use std::collections::HashMap;
use std::sync::Arc;

use cosmic::app::{Core, Task};
use cosmic::iced::Length;
use cosmic::widget;
use cosmic::Element;

use melib::backends::FlagOp;
use melib::email::Flag;
use melib::{EnvelopeHash, MailboxHash};

use crate::config::{Config, ConfigNeedsInput, FileConfig, PasswordBackend};
use crate::core::imap::ImapSession;
use crate::core::models::{Folder, MessageSummary};
use crate::core::store::{self, CacheHandle, DEFAULT_PAGE_SIZE};

const APP_ID: &str = "com.cosmic_utils.email";

pub struct AppModel {
    core: Core,
    config: Option<Config>,

    session: Option<Arc<ImapSession>>,
    cache: Option<CacheHandle>,

    folders: Vec<Folder>,
    selected_folder: Option<usize>,

    messages: Vec<MessageSummary>,
    selected_message: Option<usize>,
    messages_offset: u32,
    has_more_messages: bool,

    preview_body: String,

    /// Map folder paths (e.g. "Trash", "Archive") to mailbox hashes
    folder_map: HashMap<String, u64>,

    is_syncing: bool,
    status_message: String,

    // Setup dialog state
    show_setup_dialog: bool,
    password_only_mode: bool,
    setup_server: String,
    setup_port: String,
    setup_username: String,
    setup_password: String,
    setup_starttls: bool,
    setup_password_visible: bool,
    setup_error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Connected(Result<Arc<ImapSession>, String>),

    SelectFolder(usize),
    FoldersLoaded(Result<Vec<Folder>, String>),

    SelectMessage(usize),
    MessagesLoaded(Result<Vec<MessageSummary>, String>),

    BodyLoaded(Result<String, String>),

    // Cache-first messages
    CachedFoldersLoaded(Result<Vec<Folder>, String>),
    CachedMessagesLoaded(Result<Vec<MessageSummary>, String>),
    SyncFoldersComplete(Result<Vec<Folder>, String>),
    SyncMessagesComplete(Result<(), String>),
    LoadMoreMessages,

    // Flag/move actions
    ToggleRead(usize),
    ToggleStar(usize),
    TrashMessage(usize),
    ArchiveMessage(usize),
    FlagOpComplete {
        envelope_hash: u64,
        result: Result<u8, String>,
    },
    MoveOpComplete {
        envelope_hash: u64,
        result: Result<(), String>,
    },

    OpenLink(String),
    Refresh,
    Noop,

    // Setup dialog messages
    SetupServerChanged(String),
    SetupPortChanged(String),
    SetupUsernameChanged(String),
    SetupPasswordChanged(String),
    SetupStarttlsToggled(bool),
    SetupPasswordVisibilityToggled,
    SetupSubmit,
    SetupCancel,
}

impl cosmic::Application for AppModel {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = APP_ID;

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Self::Message>) {
        // Open cache synchronously (just opens a file, fast)
        let cache = match CacheHandle::open() {
            Ok(c) => {
                log::info!("Cache opened successfully");
                Some(c)
            }
            Err(e) => {
                log::warn!("Failed to open cache, running without: {}", e);
                None
            }
        };

        let mut app = AppModel {
            core,
            config: None,
            session: None,
            cache: cache.clone(),
            folders: Vec::new(),
            selected_folder: None,
            messages: Vec::new(),
            selected_message: None,
            messages_offset: 0,
            has_more_messages: false,
            preview_body: String::new(),
            folder_map: HashMap::new(),
            is_syncing: false,
            status_message: "Starting up...".into(),

            show_setup_dialog: false,
            password_only_mode: false,
            setup_server: String::new(),
            setup_port: "993".into(),
            setup_username: String::new(),
            setup_password: String::new(),
            setup_starttls: false,
            setup_password_visible: false,
            setup_error: None,
        };

        let title_task = app.set_window_title("Nevermail".into());
        let mut tasks = vec![title_task];

        // Load cached folders regardless of config state
        if let Some(cache) = cache.clone() {
            tasks.push(cosmic::task::future(async move {
                Message::CachedFoldersLoaded(cache.load_folders().await)
            }));
        }

        // Resolve config: env → file+keyring → show dialog
        match Config::resolve() {
            Ok(config) => {
                app.config = Some(config.clone());
                app.is_syncing = true;
                tasks.push(cosmic::task::future(async move {
                    Message::Connected(ImapSession::connect(config).await)
                }));
            }
            Err(ConfigNeedsInput::FullSetup) => {
                app.show_setup_dialog = true;
                app.password_only_mode = false;
                app.status_message = "Setup required — enter your account details".into();
            }
            Err(ConfigNeedsInput::PasswordOnly {
                server,
                port,
                username,
                starttls,
                error,
            }) => {
                app.show_setup_dialog = true;
                app.password_only_mode = true;
                app.setup_server = server;
                app.setup_port = port.to_string();
                app.setup_username = username;
                app.setup_starttls = starttls;
                app.setup_error = error;
                app.status_message = "Password required".into();
            }
        }

        (app, cosmic::task::batch(tasks))
    }

    fn dialog(&self) -> Option<Element<'_, Self::Message>> {
        if !self.show_setup_dialog {
            return None;
        }

        let mut controls = widget::column().spacing(12);

        if !self.password_only_mode {
            controls = controls
                .push(
                    widget::text_input("mail.example.com", &self.setup_server)
                        .label("IMAP Server")
                        .on_input(Message::SetupServerChanged),
                )
                .push(
                    widget::text_input("993", &self.setup_port)
                        .label("Port")
                        .on_input(Message::SetupPortChanged),
                )
                .push(
                    widget::text_input("you@example.com", &self.setup_username)
                        .label("Username")
                        .on_input(Message::SetupUsernameChanged),
                );
        }

        controls = controls.push(
            widget::text_input::secure_input(
                "Password",
                &self.setup_password,
                Some(Message::SetupPasswordVisibilityToggled),
                !self.setup_password_visible,
            )
            .label("Password")
            .on_input(Message::SetupPasswordChanged),
        );

        if !self.password_only_mode {
            controls = controls.push(
                widget::settings::item::builder("Use STARTTLS")
                    .toggler(self.setup_starttls, Message::SetupStarttlsToggled),
            );
        }

        let mut dialog = widget::dialog()
            .title(if self.password_only_mode {
                "Enter Password"
            } else {
                "Account Setup"
            })
            .control(controls)
            .primary_action(
                widget::button::suggested("Connect").on_press(Message::SetupSubmit),
            )
            .secondary_action(
                widget::button::standard("Cancel").on_press(Message::SetupCancel),
            );

        if let Some(ref err) = self.setup_error {
            dialog = dialog.body(err.as_str());
        }

        Some(dialog.into())
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let sidebar = crate::ui::sidebar::view(&self.folders, self.selected_folder);
        let message_list = crate::ui::message_list::view(
            &self.messages,
            self.selected_message,
            self.has_more_messages,
        );
        let selected_msg = self.selected_message.and_then(|i| {
            self.messages.get(i).map(|msg| (i, msg))
        });
        let message_view = crate::ui::message_view::view(&self.preview_body, selected_msg);

        let main_content = widget::row()
            .push(
                widget::container(sidebar)
                    .width(Length::FillPortion(1))
                    .height(Length::Fill),
            )
            .push(
                widget::container(message_list)
                    .width(Length::FillPortion(2))
                    .height(Length::Fill),
            )
            .push(
                widget::container(message_view)
                    .width(Length::FillPortion(3))
                    .height(Length::Fill),
            )
            .height(Length::Fill);

        let status_bar = widget::container(widget::text::caption(&self.status_message))
            .padding([4, 8])
            .width(Length::Fill);

        widget::column()
            .push(main_content)
            .push(status_bar)
            .height(Length::Fill)
            .into()
    }

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            // -----------------------------------------------------------------
            // Setup dialog input handlers
            // -----------------------------------------------------------------
            Message::SetupServerChanged(v) => {
                self.setup_server = v;
            }
            Message::SetupPortChanged(v) => {
                self.setup_port = v;
            }
            Message::SetupUsernameChanged(v) => {
                self.setup_username = v;
            }
            Message::SetupPasswordChanged(v) => {
                self.setup_password = v;
            }
            Message::SetupStarttlsToggled(v) => {
                self.setup_starttls = v;
            }
            Message::SetupPasswordVisibilityToggled => {
                self.setup_password_visible = !self.setup_password_visible;
            }

            // -----------------------------------------------------------------
            // Setup submit — validate, store credentials, connect
            // -----------------------------------------------------------------
            Message::SetupSubmit => {
                // Validate
                if self.setup_server.trim().is_empty()
                    || self.setup_username.trim().is_empty()
                    || self.setup_password.is_empty()
                {
                    self.setup_error = Some("All fields are required".into());
                    return Task::none();
                }
                let port: u16 = match self.setup_port.trim().parse() {
                    Ok(p) => p,
                    Err(_) => {
                        self.setup_error = Some("Port must be a number (e.g. 993)".into());
                        return Task::none();
                    }
                };

                let server = self.setup_server.trim().to_string();
                let username = self.setup_username.trim().to_string();
                let password = self.setup_password.clone();
                let starttls = self.setup_starttls;

                // Try keyring first; fall back to plaintext on failure
                let password_backend =
                    match crate::core::keyring::set_password(&username, &server, &password) {
                        Ok(()) => {
                            log::info!("Password stored in keyring");
                            PasswordBackend::Keyring
                        }
                        Err(e) => {
                            log::warn!("Keyring unavailable ({}), using plaintext", e);
                            PasswordBackend::Plaintext {
                                value: password.clone(),
                            }
                        }
                    };

                // Save config file
                let fc = FileConfig {
                    server: server.clone(),
                    port,
                    username: username.clone(),
                    starttls,
                    password: password_backend,
                };
                if let Err(e) = fc.save() {
                    log::error!("Failed to save config: {}", e);
                    self.setup_error = Some(format!("Failed to save config: {e}"));
                    return Task::none();
                }

                // Build runtime config and connect
                let config = Config {
                    imap_server: server,
                    imap_port: port,
                    username,
                    password,
                    use_starttls: starttls,
                };

                self.config = Some(config.clone());
                self.show_setup_dialog = false;
                self.setup_password.clear();
                self.setup_error = None;
                self.is_syncing = true;
                self.status_message = "Connecting...".into();

                return cosmic::task::future(async move {
                    Message::Connected(ImapSession::connect(config).await)
                });
            }

            // -----------------------------------------------------------------
            // Setup cancel — browse offline or show empty
            // -----------------------------------------------------------------
            Message::SetupCancel => {
                self.show_setup_dialog = false;
                if self.folders.is_empty() {
                    self.status_message = "Not connected — no cached data".into();
                } else {
                    self.status_message =
                        format!("{} folders (offline)", self.folders.len());
                }
            }

            // -----------------------------------------------------------------
            // Cache-first: cached folders loaded at startup
            // -----------------------------------------------------------------
            Message::CachedFoldersLoaded(Ok(folders)) => {
                if !folders.is_empty() {
                    self.folders = folders;
                    self.rebuild_folder_map();
                    self.status_message =
                        format!("{} folders (cached)", self.folders.len());

                    // Auto-select INBOX and load cached messages
                    if let Some(idx) = self.folders.iter().position(|f| f.path == "INBOX") {
                        self.selected_folder = Some(idx);
                        let mailbox_hash = self.folders[idx].mailbox_hash;
                        if let Some(cache) = &self.cache {
                            let cache = cache.clone();
                            self.messages_offset = 0;
                            return cosmic::task::future(async move {
                                Message::CachedMessagesLoaded(
                                    cache
                                        .load_messages(mailbox_hash, DEFAULT_PAGE_SIZE, 0)
                                        .await,
                                )
                            });
                        }
                    }
                }
            }
            Message::CachedFoldersLoaded(Err(e)) => {
                log::warn!("Failed to load cached folders: {}", e);
            }

            // -----------------------------------------------------------------
            // Cache-first: cached messages loaded
            // -----------------------------------------------------------------
            Message::CachedMessagesLoaded(Ok(messages)) => {
                let count = messages.len();
                self.has_more_messages = count as u32 == DEFAULT_PAGE_SIZE;

                if self.messages_offset == 0 {
                    self.messages = messages;
                } else {
                    self.messages.extend(messages);
                }

                if !self.messages.is_empty() {
                    self.status_message =
                        format!("{} messages", self.messages.len());
                }
            }
            Message::CachedMessagesLoaded(Err(e)) => {
                log::warn!("Failed to load cached messages: {}", e);
            }

            // -----------------------------------------------------------------
            // IMAP connected — start background folder sync
            // -----------------------------------------------------------------
            Message::Connected(Ok(session)) => {
                self.session = Some(session.clone());
                let had_cached_folders = !self.folders.is_empty();

                if !had_cached_folders {
                    self.is_syncing = true;
                    self.status_message = "Connected. Loading folders...".into();
                } else {
                    self.status_message = format!(
                        "{} folders (syncing...)",
                        self.folders.len()
                    );
                }

                let cache = self.cache.clone();
                return cosmic::task::future(async move {
                    let result = session.fetch_folders().await;
                    if let (Some(cache), Ok(ref folders)) = (&cache, &result) {
                        if let Err(e) = cache.save_folders(folders.clone()).await {
                            log::warn!("Failed to cache folders: {}", e);
                        }
                    }
                    Message::SyncFoldersComplete(result)
                });
            }
            Message::Connected(Err(e)) => {
                self.is_syncing = false;
                log::error!("IMAP connection failed: {}", e);

                if self.folders.is_empty() && !self.show_setup_dialog {
                    // No cached data and not already showing dialog — re-show with error
                    self.show_setup_dialog = true;
                    // Preserve password_only_mode from previous state if config exists,
                    // otherwise show full setup
                    if self.config.is_some() {
                        self.password_only_mode = false;
                    }
                    self.setup_error = Some(format!("Connection failed: {e}"));
                    self.status_message = format!("Connection failed: {}", e);
                } else if self.folders.is_empty() {
                    self.status_message = format!("Connection failed: {}", e);
                } else {
                    self.status_message = format!(
                        "{} folders (offline — {})",
                        self.folders.len(),
                        e
                    );
                }
            }

            // -----------------------------------------------------------------
            // Background folder sync complete
            // -----------------------------------------------------------------
            Message::SyncFoldersComplete(Ok(folders)) => {
                self.folders = folders;
                self.rebuild_folder_map();
                self.is_syncing = false;
                self.status_message = format!("{} folders", self.folders.len());

                if self.selected_folder.is_none() {
                    if let Some(idx) = self.folders.iter().position(|f| f.path == "INBOX") {
                        self.selected_folder = Some(idx);
                    }
                }

                if let Some(idx) = self.selected_folder {
                    if let Some(folder) = self.folders.get(idx) {
                        let mailbox_hash = MailboxHash(folder.mailbox_hash);
                        if let Some(session) = &self.session {
                            let session = session.clone();
                            let cache = self.cache.clone();
                            let mh = folder.mailbox_hash;
                            return cosmic::task::future(async move {
                                let result = session.fetch_messages(mailbox_hash).await;
                                if let (Some(cache), Ok(ref msgs)) = (&cache, &result) {
                                    if let Err(e) =
                                        cache.save_messages(mh, msgs.clone()).await
                                    {
                                        log::warn!("Failed to cache messages: {}", e);
                                    }
                                }
                                match result {
                                    Ok(_) => Message::SyncMessagesComplete(Ok(())),
                                    Err(e) => Message::SyncMessagesComplete(Err(e)),
                                }
                            });
                        }
                    }
                }
            }
            Message::SyncFoldersComplete(Err(e)) => {
                self.is_syncing = false;
                if self.folders.is_empty() {
                    self.status_message = format!("Failed to load folders: {}", e);
                } else {
                    self.status_message = format!(
                        "{} folders (sync failed: {})",
                        self.folders.len(),
                        e
                    );
                }
                log::error!("Folder sync failed: {}", e);
            }

            // -----------------------------------------------------------------
            // Background message sync complete — reload from cache
            // -----------------------------------------------------------------
            Message::SyncMessagesComplete(Ok(())) => {
                self.is_syncing = false;
                if let Some(idx) = self.selected_folder {
                    if let Some(folder) = self.folders.get(idx) {
                        let mailbox_hash = folder.mailbox_hash;
                        if let Some(cache) = &self.cache {
                            let cache = cache.clone();
                            self.messages_offset = 0;
                            return cosmic::task::future(async move {
                                Message::CachedMessagesLoaded(
                                    cache
                                        .load_messages(mailbox_hash, DEFAULT_PAGE_SIZE, 0)
                                        .await,
                                )
                            });
                        }
                    }
                }
                self.status_message = format!("{} messages (synced)", self.messages.len());
            }
            Message::SyncMessagesComplete(Err(e)) => {
                self.is_syncing = false;
                self.status_message = format!("Sync failed: {}", e);
                log::error!("Message sync failed: {}", e);
            }

            // -----------------------------------------------------------------
            // Legacy direct-from-server messages (used as fallback when no cache)
            // -----------------------------------------------------------------
            Message::FoldersLoaded(Ok(folders)) => {
                self.folders = folders;
                self.rebuild_folder_map();
                self.is_syncing = false;
                self.status_message = format!("{} folders loaded", self.folders.len());

                if let Some(idx) = self.folders.iter().position(|f| f.path == "INBOX") {
                    self.selected_folder = Some(idx);
                    let mailbox_hash = MailboxHash(self.folders[idx].mailbox_hash);
                    if let Some(session) = &self.session {
                        let session = session.clone();
                        self.is_syncing = true;
                        self.status_message = "Loading INBOX...".into();
                        return cosmic::task::future(async move {
                            Message::MessagesLoaded(
                                session.fetch_messages(mailbox_hash).await,
                            )
                        });
                    }
                }
            }
            Message::FoldersLoaded(Err(e)) => {
                self.is_syncing = false;
                self.status_message = format!("Failed to load folders: {}", e);
                log::error!("Folder fetch failed: {}", e);
            }

            // -----------------------------------------------------------------
            // Select folder — cache-first with background sync
            // -----------------------------------------------------------------
            Message::SelectFolder(index) => {
                self.selected_folder = Some(index);
                self.messages.clear();
                self.selected_message = None;
                self.preview_body.clear();
                self.messages_offset = 0;
                self.has_more_messages = false;

                if let Some(folder) = self.folders.get(index) {
                    let mailbox_hash = folder.mailbox_hash;
                    let folder_name = folder.name.clone();
                    let mut tasks: Vec<Task<Message>> = Vec::new();

                    if let Some(cache) = &self.cache {
                        let cache = cache.clone();
                        tasks.push(cosmic::task::future(async move {
                            Message::CachedMessagesLoaded(
                                cache.load_messages(mailbox_hash, DEFAULT_PAGE_SIZE, 0).await,
                            )
                        }));
                    }

                    if let Some(session) = &self.session {
                        let session = session.clone();
                        let cache = self.cache.clone();
                        self.is_syncing = true;
                        self.status_message = format!("Loading {}...", folder_name);
                        let mbox_hash = MailboxHash(mailbox_hash);
                        tasks.push(cosmic::task::future(async move {
                            let result = session.fetch_messages(mbox_hash).await;
                            if let (Some(cache), Ok(ref msgs)) = (&cache, &result) {
                                if let Err(e) =
                                    cache.save_messages(mailbox_hash, msgs.clone()).await
                                {
                                    log::warn!("Failed to cache messages: {}", e);
                                }
                            }
                            match result {
                                Ok(_) => Message::SyncMessagesComplete(Ok(())),
                                Err(e) => Message::SyncMessagesComplete(Err(e)),
                            }
                        }));
                    }

                    if !tasks.is_empty() {
                        return cosmic::task::batch(tasks);
                    }
                }
            }

            Message::MessagesLoaded(Ok(messages)) => {
                self.is_syncing = false;
                self.status_message = format!("{} messages", messages.len());
                self.messages = messages;
            }
            Message::MessagesLoaded(Err(e)) => {
                self.is_syncing = false;
                self.status_message = format!("Failed to load messages: {}", e);
                log::error!("Message fetch failed: {}", e);
            }

            // -----------------------------------------------------------------
            // Load more messages (pagination)
            // -----------------------------------------------------------------
            Message::LoadMoreMessages => {
                self.messages_offset += DEFAULT_PAGE_SIZE;
                let offset = self.messages_offset;

                if let Some(idx) = self.selected_folder {
                    if let Some(folder) = self.folders.get(idx) {
                        let mailbox_hash = folder.mailbox_hash;
                        if let Some(cache) = &self.cache {
                            let cache = cache.clone();
                            return cosmic::task::future(async move {
                                Message::CachedMessagesLoaded(
                                    cache
                                        .load_messages(mailbox_hash, DEFAULT_PAGE_SIZE, offset)
                                        .await,
                                )
                            });
                        }
                    }
                }
            }

            // -----------------------------------------------------------------
            // Select message — cache-first body loading
            // -----------------------------------------------------------------
            Message::SelectMessage(index) => {
                self.selected_message = Some(index);

                if let Some(msg) = self.messages.get(index) {
                    let envelope_hash = msg.envelope_hash;

                    if let Some(cache) = &self.cache {
                        let cache = cache.clone();
                        let session = self.session.clone();
                        self.status_message = "Loading message...".into();
                        return cosmic::task::future(async move {
                            match cache.load_body(envelope_hash).await {
                                Ok(Some(body)) => Message::BodyLoaded(Ok(body)),
                                _ => {
                                    if let Some(session) = session {
                                        let result = session
                                            .fetch_body(EnvelopeHash(envelope_hash))
                                            .await;
                                        if let Ok(ref body) = result {
                                            if let Err(e) = cache
                                                .save_body(envelope_hash, body.clone())
                                                .await
                                            {
                                                log::warn!(
                                                    "Failed to cache body: {}",
                                                    e
                                                );
                                            }
                                        }
                                        Message::BodyLoaded(result)
                                    } else {
                                        Message::BodyLoaded(Err(
                                            "Not connected".to_string()
                                        ))
                                    }
                                }
                            }
                        });
                    }

                    if let Some(session) = &self.session {
                        let session = session.clone();
                        self.status_message = "Loading message...".into();
                        return cosmic::task::future(async move {
                            Message::BodyLoaded(
                                session.fetch_body(EnvelopeHash(envelope_hash)).await,
                            )
                        });
                    }
                }
            }

            Message::BodyLoaded(Ok(body)) => {
                self.preview_body = body;
                self.status_message = "Ready".into();
            }
            Message::BodyLoaded(Err(e)) => {
                self.preview_body = format!("Failed to load message body: {}", e);
                self.status_message = "Error loading message".into();
                log::error!("Body fetch failed: {}", e);
            }

            // -----------------------------------------------------------------
            // Flag actions — optimistic UI + background IMAP op
            // -----------------------------------------------------------------
            Message::ToggleRead(index) => {
                if let Some(msg) = self.messages.get_mut(index) {
                    let new_read = !msg.is_read;
                    msg.is_read = new_read;
                    let envelope_hash = msg.envelope_hash;
                    let mailbox_hash = msg.mailbox_hash;
                    let new_flags = store::flags_to_u8(new_read, msg.is_starred);
                    let pending_op = if new_read { "set_seen" } else { "unset_seen" }.to_string();

                    let mut tasks: Vec<Task<Message>> = Vec::new();

                    if let Some(cache) = &self.cache {
                        let cache = cache.clone();
                        let op = pending_op.clone();
                        tasks.push(cosmic::task::future(async move {
                            if let Err(e) = cache.update_flags(envelope_hash, new_flags, op).await {
                                log::warn!("Failed to update cache flags: {}", e);
                            }
                            Message::Noop
                        }));
                    }

                    if let Some(session) = &self.session {
                        let session = session.clone();
                        let flag_op = if new_read {
                            FlagOp::Set(Flag::SEEN)
                        } else {
                            FlagOp::UnSet(Flag::SEEN)
                        };
                        tasks.push(cosmic::task::future(async move {
                            let result = session
                                .set_flags(
                                    EnvelopeHash(envelope_hash),
                                    MailboxHash(mailbox_hash),
                                    vec![flag_op],
                                )
                                .await;
                            Message::FlagOpComplete {
                                envelope_hash,
                                result: result.map(|_| new_flags),
                            }
                        }));
                    }

                    if !tasks.is_empty() {
                        return cosmic::task::batch(tasks);
                    }
                }
            }

            Message::ToggleStar(index) => {
                if let Some(msg) = self.messages.get_mut(index) {
                    let new_starred = !msg.is_starred;
                    msg.is_starred = new_starred;
                    let envelope_hash = msg.envelope_hash;
                    let mailbox_hash = msg.mailbox_hash;
                    let new_flags = store::flags_to_u8(msg.is_read, new_starred);
                    let pending_op = if new_starred { "set_flagged" } else { "unset_flagged" }.to_string();

                    let mut tasks: Vec<Task<Message>> = Vec::new();

                    if let Some(cache) = &self.cache {
                        let cache = cache.clone();
                        let op = pending_op.clone();
                        tasks.push(cosmic::task::future(async move {
                            if let Err(e) = cache.update_flags(envelope_hash, new_flags, op).await {
                                log::warn!("Failed to update cache flags: {}", e);
                            }
                            Message::Noop
                        }));
                    }

                    if let Some(session) = &self.session {
                        let session = session.clone();
                        let flag_op = if new_starred {
                            FlagOp::Set(Flag::FLAGGED)
                        } else {
                            FlagOp::UnSet(Flag::FLAGGED)
                        };
                        tasks.push(cosmic::task::future(async move {
                            let result = session
                                .set_flags(
                                    EnvelopeHash(envelope_hash),
                                    MailboxHash(mailbox_hash),
                                    vec![flag_op],
                                )
                                .await;
                            Message::FlagOpComplete {
                                envelope_hash,
                                result: result.map(|_| new_flags),
                            }
                        }));
                    }

                    if !tasks.is_empty() {
                        return cosmic::task::batch(tasks);
                    }
                }
            }

            Message::TrashMessage(index) => {
                if let Some(trash_hash) = self.folder_map.get("Trash").or_else(|| self.folder_map.get("INBOX.Trash")).copied() {
                    if let Some(msg) = self.messages.get(index) {
                        let envelope_hash = msg.envelope_hash;
                        let source_mailbox = msg.mailbox_hash;

                        // Optimistic: remove from list
                        self.messages.remove(index);
                        if let Some(sel) = &mut self.selected_message {
                            if *sel >= self.messages.len() && !self.messages.is_empty() {
                                *sel = self.messages.len() - 1;
                            } else if self.messages.is_empty() {
                                self.selected_message = None;
                                self.preview_body.clear();
                            }
                        }

                        let mut tasks: Vec<Task<Message>> = Vec::new();

                        if let Some(cache) = &self.cache {
                            let cache = cache.clone();
                            let new_flags = store::flags_to_u8(true, false);
                            tasks.push(cosmic::task::future(async move {
                                if let Err(e) = cache.update_flags(envelope_hash, new_flags, format!("move:{}", trash_hash)).await {
                                    log::warn!("Failed to update cache for trash: {}", e);
                                }
                                Message::Noop
                            }));
                        }

                        if let Some(session) = &self.session {
                            let session = session.clone();
                            tasks.push(cosmic::task::future(async move {
                                let result = session
                                    .move_messages(
                                        EnvelopeHash(envelope_hash),
                                        MailboxHash(source_mailbox),
                                        MailboxHash(trash_hash),
                                    )
                                    .await;
                                Message::MoveOpComplete {
                                    envelope_hash,
                                    result,
                                }
                            }));
                        }

                        if !tasks.is_empty() {
                            return cosmic::task::batch(tasks);
                        }
                    }
                } else {
                    self.status_message = "Trash folder not found".into();
                }
            }

            Message::ArchiveMessage(index) => {
                if let Some(archive_hash) = self.folder_map.get("Archive").or_else(|| self.folder_map.get("INBOX.Archive")).copied() {
                    if let Some(msg) = self.messages.get(index) {
                        let envelope_hash = msg.envelope_hash;
                        let source_mailbox = msg.mailbox_hash;

                        // Optimistic: remove from list
                        self.messages.remove(index);
                        if let Some(sel) = &mut self.selected_message {
                            if *sel >= self.messages.len() && !self.messages.is_empty() {
                                *sel = self.messages.len() - 1;
                            } else if self.messages.is_empty() {
                                self.selected_message = None;
                                self.preview_body.clear();
                            }
                        }

                        let mut tasks: Vec<Task<Message>> = Vec::new();

                        if let Some(cache) = &self.cache {
                            let cache = cache.clone();
                            let new_flags = store::flags_to_u8(true, false);
                            tasks.push(cosmic::task::future(async move {
                                if let Err(e) = cache.update_flags(envelope_hash, new_flags, format!("move:{}", archive_hash)).await {
                                    log::warn!("Failed to update cache for archive: {}", e);
                                }
                                Message::Noop
                            }));
                        }

                        if let Some(session) = &self.session {
                            let session = session.clone();
                            tasks.push(cosmic::task::future(async move {
                                let result = session
                                    .move_messages(
                                        EnvelopeHash(envelope_hash),
                                        MailboxHash(source_mailbox),
                                        MailboxHash(archive_hash),
                                    )
                                    .await;
                                Message::MoveOpComplete {
                                    envelope_hash,
                                    result,
                                }
                            }));
                        }

                        if !tasks.is_empty() {
                            return cosmic::task::batch(tasks);
                        }
                    }
                } else {
                    self.status_message = "Archive folder not found".into();
                }
            }

            // -----------------------------------------------------------------
            // Background flag/move op results
            // -----------------------------------------------------------------
            Message::FlagOpComplete {
                envelope_hash,
                result,
            } => {
                match result {
                    Ok(new_flags) => {
                        if let Some(cache) = &self.cache {
                            let cache = cache.clone();
                            return cosmic::task::future(async move {
                                if let Err(e) = cache.clear_pending_op(envelope_hash, new_flags).await {
                                    log::warn!("Failed to clear pending op: {}", e);
                                }
                                Message::Noop
                            });
                        }
                    }
                    Err(e) => {
                        log::error!("Flag operation failed: {}", e);
                        self.status_message = format!("Flag update failed: {}", e);

                        // Revert optimistic UI
                        if let Some(msg) = self.messages.iter_mut().find(|m| m.envelope_hash == envelope_hash) {
                            msg.is_read = !msg.is_read; // toggle back
                        }

                        if let Some(cache) = &self.cache {
                            let cache = cache.clone();
                            return cosmic::task::future(async move {
                                if let Err(e) = cache.revert_pending_op(envelope_hash).await {
                                    log::warn!("Failed to revert pending op: {}", e);
                                }
                                Message::Noop
                            });
                        }
                    }
                }
            }

            Message::MoveOpComplete {
                envelope_hash,
                result,
            } => {
                match result {
                    Ok(()) => {
                        if let Some(cache) = &self.cache {
                            let cache = cache.clone();
                            return cosmic::task::future(async move {
                                if let Err(e) = cache.remove_message(envelope_hash).await {
                                    log::warn!("Failed to remove message from cache: {}", e);
                                }
                                Message::Noop
                            });
                        }
                    }
                    Err(e) => {
                        log::error!("Move operation failed: {}", e);
                        self.status_message = format!("Move failed: {}", e);
                        // TODO: re-insert message on failure (would need to store removed msg)
                        // For now, a refresh will restore correct state
                    }
                }
            }

            Message::OpenLink(url) => {
                crate::core::mime::open_link(&url);
            }
            Message::Refresh => {
                if let Some(session) = &self.session {
                    let session = session.clone();
                    let cache = self.cache.clone();
                    self.is_syncing = true;
                    self.status_message = "Refreshing...".into();
                    return cosmic::task::future(async move {
                        let result = session.fetch_folders().await;
                        if let (Some(cache), Ok(ref folders)) = (&cache, &result) {
                            if let Err(e) = cache.save_folders(folders.clone()).await {
                                log::warn!("Failed to cache folders: {}", e);
                            }
                        }
                        Message::SyncFoldersComplete(result)
                    });
                }
            }
            Message::Noop => {}
        }
        Task::none()
    }
}

impl AppModel {
    fn set_window_title(&self, title: String) -> cosmic::app::Task<Message> {
        self.core.set_title(self.core.main_window_id(), title)
    }

    /// Rebuild folder_map from current folders list.
    fn rebuild_folder_map(&mut self) {
        self.folder_map.clear();
        for f in &self.folders {
            self.folder_map.insert(f.path.clone(), f.mailbox_hash);
        }
    }
}
