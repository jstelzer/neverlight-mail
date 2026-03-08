//! Sync dispatcher — routes sync-related messages to handler methods.
//!
//! Pure helper functions and the thin `handle_sync` dispatcher live here.
//! Handler implementations live in `sync_apply.rs`.

use cosmic::app::Task;
use neverlight_mail_core::store::DEFAULT_PAGE_SIZE;
use std::time::{Duration, Instant};

use super::{AppModel, Message, Phase};

struct CachedMessagesContext<'a> {
    epoch: u64,
    account_id: Option<&'a str>,
    mailbox_id: Option<&'a str>,
    offset: u32,
}

fn should_apply_cached_messages(
    current: &CachedMessagesContext<'_>,
    incoming: &CachedMessagesContext<'_>,
) -> bool {
    current.epoch == incoming.epoch
        && current.account_id == incoming.account_id
        && current.mailbox_id == incoming.mailbox_id
        && current.offset == incoming.offset
}

pub(super) fn should_queue_refresh(refresh_in_flight: bool) -> bool {
    refresh_in_flight
}

pub(super) fn mark_refresh_account_complete(
    outstanding: &mut std::collections::HashSet<String>,
    account_id: &str,
) -> bool {
    outstanding.remove(account_id);
    outstanding.is_empty()
}

pub(super) const REFRESH_STUCK_TIMEOUT: Duration = Duration::from_secs(45);

pub(super) fn refresh_has_timed_out(
    refresh_started_at: Option<Instant>,
    refresh_timeout_reported: bool,
) -> bool {
    if refresh_timeout_reported {
        return false;
    }
    refresh_started_at.is_some_and(|started| started.elapsed() >= REFRESH_STUCK_TIMEOUT)
}

impl AppModel {
    pub(super) fn handle_sync(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::CachedFoldersLoaded { account_id, result: Ok(folders) } => {
                return self.handle_cached_folders_ok(account_id, folders);
            }
            Message::CachedFoldersLoaded { result: Err(e), .. } => {
                log::warn!("Failed to load cached folders: {}", e);
            }

            Message::CachedMessagesLoaded {
                account_id,
                ref mailbox_id,
                offset,
                epoch,
                result: Ok(messages),
            } => {
                let active_account_id = self
                    .active_account
                    .and_then(|i| self.accounts.get(i))
                    .map(|a| a.config.id.as_str());
                let active_mailbox_id = self.selected_folder.and_then(|fi| {
                    self.active_account
                        .and_then(|ai| self.accounts.get(ai))
                        .and_then(|a| a.folders.get(fi))
                        .map(|f| f.mailbox_id.as_str())
                });
                let current = CachedMessagesContext {
                    epoch: self.folder_epoch,
                    account_id: active_account_id,
                    mailbox_id: active_mailbox_id,
                    offset: self.messages_offset,
                };
                let incoming = CachedMessagesContext {
                    epoch,
                    account_id: Some(account_id.as_str()),
                    mailbox_id: Some(mailbox_id.as_str()),
                    offset,
                };
                if !should_apply_cached_messages(&current, &incoming) {
                    self.stale_apply_drop_count = self.stale_apply_drop_count.saturating_add(1);
                    return Task::none();
                }

                let count = messages.len();
                self.has_more_messages = count as u32 == DEFAULT_PAGE_SIZE;
                self.folder_abort = None;

                let prev_email_id = self.selected_message.and_then(|i| {
                    self.messages.get(i).map(|m| m.email_id.clone())
                });

                if self.messages_offset == 0 {
                    self.messages = messages;
                } else {
                    self.messages.extend(messages);
                }

                if self.messages_offset == 0 {
                    if let Some(ref eid) = prev_email_id {
                        self.selected_message = self
                            .messages
                            .iter()
                            .position(|m| m.email_id == *eid);
                    }
                }

                self.recompute_visible();

                if self.messages_offset == 0 {
                    self.reconcile_folder_unread_count(&account_id, mailbox_id);
                }

                if !self.messages.is_empty() {
                    self.status_message =
                        format!("{} messages", self.messages.len());
                }
                self.phase = Phase::Idle;
            }
            Message::CachedMessagesLoaded { epoch, result: Err(e), .. } => {
                if epoch != self.folder_epoch {
                    self.stale_apply_drop_count = self.stale_apply_drop_count.saturating_add(1);
                    return Task::none();
                }
                self.folder_abort = None;
                log::warn!("Failed to load cached messages: {}", e);
            }

            Message::AccountConnected { account_id, result: Ok(client) } => {
                return self.handle_account_connected_ok(account_id, client);
            }
            Message::AccountConnected { account_id, result: Err(e) } => {
                return self.handle_account_connected_err(account_id, e);
            }

            Message::SyncFoldersComplete {
                account_id,
                epoch,
                result: Ok(folders),
            } => {
                return self.handle_sync_folders_ok(account_id, epoch, folders);
            }
            Message::SyncFoldersComplete {
                account_id,
                epoch,
                result: Err(e),
            } => {
                return self.handle_sync_folders_err(account_id, epoch, e);
            }

            Message::SyncMessagesComplete {
                account_id,
                ref mailbox_id,
                epoch,
                result: Ok(()),
            } => {
                return self.handle_sync_messages_ok(account_id, mailbox_id.clone(), epoch);
            }
            Message::SyncMessagesComplete { ref account_id, epoch, result: Err(ref e), .. } => {
                return self.handle_sync_messages_err(account_id, epoch, e);
            }

            Message::SelectFolder(acct_idx, folder_idx) => {
                return self.handle_select_folder(acct_idx, folder_idx);
            }

            Message::LoadMoreMessages => {
                return self.handle_load_more_messages();
            }

            Message::Refresh => {
                return self.handle_refresh();
            }

            Message::ForceReconnect(ref account_id) => {
                return self.handle_force_reconnect(account_id);
            }

            _ => {}
        }
        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        mark_refresh_account_complete, refresh_has_timed_out, should_apply_cached_messages,
        should_queue_refresh, CachedMessagesContext,
    };
    use std::collections::HashSet;
    use std::time::{Duration, Instant};

    fn ctx<'a>(
        epoch: u64,
        account_id: Option<&'a str>,
        mailbox_id: Option<&'a str>,
        offset: u32,
    ) -> CachedMessagesContext<'a> {
        CachedMessagesContext { epoch, account_id, mailbox_id, offset }
    }

    #[test]
    fn cached_messages_apply_when_epoch_and_context_match() {
        let current = ctx(3, Some("acct-1"), Some("mbox-42"), 0);
        let incoming = ctx(3, Some("acct-1"), Some("mbox-42"), 0);
        assert!(should_apply_cached_messages(&current, &incoming));
    }

    #[test]
    fn cached_messages_drop_on_epoch_mismatch() {
        let current = ctx(3, Some("acct-1"), Some("mbox-42"), 0);
        let incoming = ctx(2, Some("acct-1"), Some("mbox-42"), 0);
        assert!(!should_apply_cached_messages(&current, &incoming));
    }

    #[test]
    fn cached_messages_drop_on_account_or_mailbox_or_offset_mismatch() {
        let base = ctx(3, Some("acct-1"), Some("mbox-42"), 0);

        let wrong_account = ctx(3, Some("acct-2"), Some("mbox-42"), 0);
        assert!(!should_apply_cached_messages(&wrong_account, &base));

        let wrong_mailbox = ctx(3, Some("acct-1"), Some("mbox-7"), 0);
        assert!(!should_apply_cached_messages(&wrong_mailbox, &base));

        let wrong_offset = ctx(3, Some("acct-1"), Some("mbox-42"), 50);
        assert!(!should_apply_cached_messages(&wrong_offset, &base));
    }

    #[test]
    fn refresh_is_queued_when_in_flight() {
        assert!(should_queue_refresh(true));
        assert!(!should_queue_refresh(false));
    }

    #[test]
    fn refresh_completion_drains_outstanding_accounts() {
        let mut outstanding: HashSet<String> =
            ["acct-a".to_string(), "acct-b".to_string()].into_iter().collect();
        assert!(!mark_refresh_account_complete(&mut outstanding, "acct-a"));
        assert_eq!(outstanding.len(), 1);
        assert!(mark_refresh_account_complete(&mut outstanding, "acct-b"));
        assert!(outstanding.is_empty());
    }

    #[test]
    fn refresh_timeout_detects_stuck_once_per_cycle() {
        let started = Some(Instant::now() - Duration::from_secs(60));
        assert!(refresh_has_timed_out(started, false));
        assert!(!refresh_has_timed_out(started, true));
        assert!(!refresh_has_timed_out(Some(Instant::now()), false));
    }
}
