use std::sync::Arc;

use cosmic::app::{Core, Task};
use cosmic::iced::Length;
use cosmic::widget;
use cosmic::Element;

use melib::{EnvelopeHash, MailboxHash};

use crate::config::Config;
use crate::core::imap::ImapSession;
use crate::core::models::{Folder, MessageSummary};

const APP_ID: &str = "com.cosmic_utils.email";

pub struct AppModel {
    core: Core,
    config: Config,

    session: Option<Arc<ImapSession>>,

    folders: Vec<Folder>,
    selected_folder: Option<usize>,

    messages: Vec<MessageSummary>,
    selected_message: Option<usize>,

    preview_body: String,

    is_syncing: bool,
    status_message: String,
}

#[derive(Debug, Clone)]
pub enum Message {
    Connected(Result<Arc<ImapSession>, String>),

    SelectFolder(usize),
    FoldersLoaded(Result<Vec<Folder>, String>),

    SelectMessage(usize),
    MessagesLoaded(Result<Vec<MessageSummary>, String>),

    BodyLoaded(Result<String, String>),

    OpenLink(String),
    Refresh,
    Noop,
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
        let config = Config::from_env();

        let app = AppModel {
            core,
            config: config.clone(),
            session: None,
            folders: Vec::new(),
            selected_folder: None,
            messages: Vec::new(),
            selected_message: None,
            preview_body: String::new(),
            is_syncing: true,
            status_message: "Connecting...".into(),
        };

        let title_task = app.set_window_title("Nevermail".into());

        let connect_task = cosmic::task::future(async move {
            Message::Connected(ImapSession::connect(config).await)
        });

        (app, cosmic::task::batch([title_task, connect_task]))
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let sidebar = crate::ui::sidebar::view(&self.folders, self.selected_folder);
        let message_list = crate::ui::message_list::view(&self.messages, self.selected_message);
        let message_view = crate::ui::message_view::view(&self.preview_body);

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
            Message::Connected(Ok(session)) => {
                self.session = Some(session.clone());
                self.is_syncing = true;
                self.status_message = "Connected. Loading folders...".into();

                return cosmic::task::future(async move {
                    Message::FoldersLoaded(session.fetch_folders().await)
                });
            }
            Message::Connected(Err(e)) => {
                self.is_syncing = false;
                self.status_message = format!("Connection failed: {}", e);
                log::error!("IMAP connection failed: {}", e);
            }

            Message::FoldersLoaded(Ok(folders)) => {
                self.folders = folders;
                self.is_syncing = false;
                self.status_message = format!("{} folders loaded", self.folders.len());

                // Auto-select INBOX if present
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

            Message::SelectFolder(index) => {
                self.selected_folder = Some(index);
                self.messages.clear();
                self.selected_message = None;
                self.preview_body.clear();

                if let Some(folder) = self.folders.get(index) {
                    let mailbox_hash = MailboxHash(folder.mailbox_hash);
                    if let Some(session) = &self.session {
                        let session = session.clone();
                        self.is_syncing = true;
                        self.status_message = format!("Loading {}...", folder.name);
                        return cosmic::task::future(async move {
                            Message::MessagesLoaded(
                                session.fetch_messages(mailbox_hash).await,
                            )
                        });
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

            Message::SelectMessage(index) => {
                self.selected_message = Some(index);

                if let Some(msg) = self.messages.get(index) {
                    let envelope_hash = EnvelopeHash(msg.envelope_hash);
                    if let Some(session) = &self.session {
                        let session = session.clone();
                        self.status_message = "Loading message...".into();
                        return cosmic::task::future(async move {
                            Message::BodyLoaded(session.fetch_body(envelope_hash).await)
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

            Message::OpenLink(url) => {
                crate::core::mime::open_link(&url);
            }
            Message::Refresh => {
                if let Some(session) = &self.session {
                    let session = session.clone();
                    self.is_syncing = true;
                    self.status_message = "Refreshing folders...".into();
                    return cosmic::task::future(async move {
                        Message::FoldersLoaded(session.fetch_folders().await)
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
}
