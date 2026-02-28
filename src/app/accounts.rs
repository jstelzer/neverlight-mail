use std::collections::HashMap;
use std::sync::Arc;

use cosmic::app::Task;

use neverlight_mail_core::config::ConfigNeedsInput;
use neverlight_mail_core::imap::ImapSession;
use neverlight_mail_core::setup::SetupModel;

use super::{AppModel, Message};

impl AppModel {
    /// Find the account index that owns a given mailbox_hash.
    pub(super) fn account_for_mailbox(&self, mailbox_hash: u64) -> Option<usize> {
        self.accounts.iter().position(|a| {
            a.folders.iter().any(|f| f.mailbox_hash == mailbox_hash)
        })
    }

    /// Get the session for a given mailbox_hash.
    pub(super) fn session_for_mailbox(&self, mailbox_hash: u64) -> Option<Arc<ImapSession>> {
        self.account_for_mailbox(mailbox_hash)
            .and_then(|i| self.accounts[i].session.clone())
    }

    /// Get the folder_map for a given mailbox_hash's owning account.
    pub(super) fn folder_map_for_mailbox(&self, mailbox_hash: u64) -> Option<&HashMap<String, u64>> {
        self.account_for_mailbox(mailbox_hash)
            .map(|i| &self.accounts[i].folder_map)
    }

    /// Get the active account's ID, or empty string.
    pub(super) fn active_account_id(&self) -> String {
        self.active_account
            .and_then(|i| self.accounts.get(i))
            .map(|a| a.config.id.clone())
            .unwrap_or_default()
    }

    /// Get the active account's session.
    pub(super) fn active_session(&self) -> Option<Arc<ImapSession>> {
        self.active_account
            .and_then(|i| self.accounts.get(i))
            .and_then(|a| a.session.clone())
    }

    /// Find account index by ID.
    pub(super) fn account_index(&self, account_id: &str) -> Option<usize> {
        self.accounts.iter().position(|a| a.config.id == account_id)
    }

    /// Refresh the cached compose labels (account labels + from addresses)
    /// so dialog() can borrow them with &self lifetime.
    pub(super) fn refresh_compose_cache(&mut self) {
        self.compose_account_labels = self.accounts.iter().map(|a| a.config.label.clone()).collect();
        self.compose_cached_from = self
            .accounts
            .get(self.compose_account)
            .map(|a| a.config.email_addresses.clone())
            .unwrap_or_default();
    }

    /// Handle account management messages (add/edit/remove/collapse).
    pub(super) fn handle_account_management(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::AccountAdd => {
                self.setup_model = Some(SetupModel::from_config_needs(&ConfigNeedsInput::FullSetup));
                self.setup_password_visible = false;
            }
            Message::AccountEdit(ref id) => {
                if let Some(acct) = self.accounts.iter().find(|a| &a.config.id == id) {
                    use neverlight_mail_core::setup::SetupFields;
                    self.setup_model = Some(SetupModel::for_edit(
                        id.clone(),
                        SetupFields {
                            label: acct.config.label.clone(),
                            server: acct.config.imap_server.clone(),
                            port: acct.config.imap_port.to_string(),
                            username: acct.config.username.clone(),
                            email: acct.config.email_addresses.join(", "),
                            starttls: acct.config.use_starttls,
                            smtp_server: acct.config.smtp_overrides.server.clone().unwrap_or_default(),
                            smtp_port: acct.config.smtp_overrides.port.map(|p| p.to_string()).unwrap_or_else(|| "587".into()),
                            smtp_username: acct.config.smtp_overrides.username.clone().unwrap_or_default(),
                            smtp_starttls: acct.config.smtp_overrides.use_starttls.unwrap_or(true),
                        },
                    ));
                    self.setup_password_visible = false;
                }
            }
            Message::AccountRemove(ref id) => {
                if let Some(idx) = self.account_index(id) {
                    let removed_id = self.accounts[idx].config.id.clone();
                    let removed_username = self.accounts[idx].config.username.clone();
                    let removed_server = self.accounts[idx].config.imap_server.clone();
                    self.accounts.remove(idx);
                    // Adjust active_account
                    if let Some(active) = self.active_account {
                        if active == idx {
                            self.active_account = None;
                            self.messages.clear();
                            self.selected_folder = None;
                            self.preview_body.clear();
                            self.preview_markdown.clear();
                        } else if active > idx {
                            self.active_account = Some(active - 1);
                        }
                    }
                    // Save updated config
                    let _ = self.save_multi_account_config();

                    // Clean up keyring passwords
                    if let Err(e) = neverlight_mail_core::keyring::delete_password(&removed_username, &removed_server) {
                        log::warn!("Failed to delete IMAP password from keyring: {}", e);
                    }
                    if let Err(e) = neverlight_mail_core::keyring::delete_smtp_password(&removed_id) {
                        log::debug!("No SMTP password to delete from keyring: {}", e);
                    }

                    self.status_message = "Account removed".into();

                    // Clean up cached data for removed account
                    if let Some(cache) = &self.cache {
                        let cache = cache.clone();
                        return cosmic::task::future(async move {
                            if let Err(e) = cache.remove_account(removed_id).await {
                                log::warn!("Failed to clean cache for removed account: {}", e);
                            }
                            Message::Noop
                        });
                    }
                }
            }
            Message::ToggleAccountCollapse(idx) => {
                if let Some(acct) = self.accounts.get_mut(idx) {
                    acct.collapsed = !acct.collapsed;
                }
            }
            _ => {}
        }
        Task::none()
    }

    /// Save the current account list to the multi-account config file.
    pub(super) fn save_multi_account_config(&self) -> Result<(), String> {
        use neverlight_mail_core::config::{FileAccountConfig, MultiAccountFileConfig, PasswordBackend};

        let accounts: Vec<FileAccountConfig> = self
            .accounts
            .iter()
            .map(|a| FileAccountConfig {
                id: a.config.id.clone(),
                label: a.config.label.clone(),
                server: a.config.imap_server.clone(),
                port: a.config.imap_port,
                username: a.config.username.clone(),
                starttls: a.config.use_starttls,
                password: PasswordBackend::Keyring,
                email_addresses: a.config.email_addresses.clone(),
                smtp: a.config.smtp_overrides.clone(),
            })
            .collect();

        let config = MultiAccountFileConfig { accounts };
        config.save()
    }
}
