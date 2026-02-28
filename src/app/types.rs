use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use cosmic::app::Core;
use cosmic::widget::{image, markdown, pane_grid, text_editor};

use neverlight_mail_core::config::{AccountConfig, AccountId};
use neverlight_mail_core::imap::ImapSession;
use neverlight_mail_core::models::{AttachmentData, Folder, MessageSummary};
use neverlight_mail_core::setup::SetupModel;
use neverlight_mail_core::store::CacheHandle;

use crate::dnd_models::DraggedFiles;
use crate::ui::compose_dialog::ComposeMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneKind {
    Sidebar,
    MessageList,
    MessageView,
}

pub(crate) const APP_ID: &str = "com.neverlight.email";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Syncing,
    Error(String),
}

// ---------------------------------------------------------------------------
// Per-account state
// ---------------------------------------------------------------------------

pub struct AccountState {
    pub config: AccountConfig,
    pub session: Option<Arc<ImapSession>>,
    pub conn_state: ConnectionState,
    pub folders: Vec<Folder>,
    pub folder_map: HashMap<String, u64>,
    pub collapsed: bool,
}

impl AccountState {
    pub fn new(config: AccountConfig) -> Self {
        AccountState {
            config,
            session: None,
            conn_state: ConnectionState::Disconnected,
            folders: Vec::new(),
            folder_map: HashMap::new(),
            collapsed: false,
        }
    }

    pub fn rebuild_folder_map(&mut self) {
        self.folder_map.clear();
        for f in &self.folders {
            self.folder_map.insert(f.path.clone(), f.mailbox_hash);
        }
    }
}

// ---------------------------------------------------------------------------
// AppModel
// ---------------------------------------------------------------------------

pub struct AppModel {
    pub(crate) core: Core,

    // Multi-account state
    pub(super) accounts: Vec<AccountState>,
    pub(super) active_account: Option<usize>,

    pub(super) cache: Option<CacheHandle>,

    pub(super) selected_folder: Option<usize>,

    pub(super) messages: Vec<MessageSummary>,
    pub(super) selected_message: Option<usize>,
    pub(super) messages_offset: u32,
    pub(super) has_more_messages: bool,

    pub(super) preview_body: String,
    pub(super) preview_markdown: Vec<markdown::Item>,
    pub(super) preview_attachments: Vec<AttachmentData>,
    pub(super) preview_image_handles: Vec<Option<image::Handle>>,

    /// Thread IDs that are currently collapsed (children hidden)
    pub(super) collapsed_threads: HashSet<u64>,
    /// Maps visible row positions → real indices into `messages`
    pub(super) visible_indices: Vec<usize>,
    /// Total messages per thread_id (for collapse indicators)
    pub(super) thread_sizes: HashMap<u64, usize>,
    /// Snapshot of optimistically removed messages for move rollback.
    pub(super) pending_move_restore: HashMap<u64, (MessageSummary, usize)>,

    pub(super) status_message: String,

    // Search state
    pub(super) search_active: bool,
    pub(super) search_query: String,
    pub(super) search_focused: bool,

    // Compose dialog state
    pub(super) show_compose_dialog: bool,
    pub(super) compose_mode: ComposeMode,
    pub(super) compose_account: usize,
    pub(super) compose_from: usize,
    pub(super) compose_to: String,
    pub(super) compose_subject: String,
    pub(super) compose_body: text_editor::Content,
    pub(super) compose_in_reply_to: Option<String>,
    pub(super) compose_references: Option<String>,
    pub(super) compose_attachments: Vec<AttachmentData>,
    pub(super) compose_error: Option<String>,
    pub(super) compose_drag_hover: bool,
    pub(super) is_sending: bool,
    // Cached for dialog() lifetime (updated when compose_account changes)
    pub(super) compose_account_labels: Vec<String>,
    pub(super) compose_cached_from: Vec<String>,

    // Setup dialog state — core fields live in SetupModel, visibility is local
    pub(super) setup_model: Option<SetupModel>,
    pub(super) setup_password_visible: bool,

    // DnD state
    pub(super) folder_drag_target: Option<usize>,

    /// Body view deferred until IMAP session is ready
    pub(super) pending_body: Option<usize>,
    /// Retry count for deferred body fetches (prevents infinite loops)
    pub(super) body_defer_retries: u8,

    /// Auto-mark-read: suppressed when user manually toggles back to unread
    pub(super) auto_read_suppressed: bool,

    // Pane layout
    pub(super) panes: pane_grid::State<PaneKind>,
}

#[derive(Debug, Clone)]
pub enum Message {
    AccountConnected {
        account_id: AccountId,
        result: Result<Arc<ImapSession>, String>,
    },

    SelectFolder(usize, usize), // (account_idx, folder_idx)

    ViewBody(usize),
    BodyDeferred,
    BodyLoaded(Result<(String, String, Vec<AttachmentData>), String>),
    LinkClicked(markdown::Url),
    CopyBody,

    SaveAttachment(usize),
    SaveAttachmentComplete(Result<String, String>),

    // Cache-first messages
    CachedFoldersLoaded {
        account_id: AccountId,
        result: Result<Vec<Folder>, String>,
    },
    CachedMessagesLoaded(Result<Vec<MessageSummary>, String>),
    SyncFoldersComplete {
        account_id: AccountId,
        result: Result<Vec<Folder>, String>,
    },
    SyncMessagesComplete(Result<(), String>),
    LoadMoreMessages,

    // Flag/move actions
    ToggleRead(usize),
    ToggleStar(usize),
    Trash(usize),
    Archive(usize),
    FlagOpComplete {
        envelope_hash: u64,
        prev_flags: u8,
        result: Result<u8, String>,
    },
    MoveOpComplete {
        envelope_hash: u64,
        result: Result<(), String>,
    },

    // Keyboard navigation
    SelectionUp,
    SelectionDown,
    ActivateSelection,
    ToggleThreadCollapse,

    // Compose messages
    ComposeNew,
    ComposeReply,
    ComposeForward,
    ComposeAccountChanged(usize),
    ComposeFromChanged(usize),
    ComposeToChanged(String),
    ComposeSubjectChanged(String),
    ComposeBodyAction(text_editor::Action),
    ComposeAttach,
    ComposeAttachLoaded(Result<Vec<AttachmentData>, String>),
    ComposeRemoveAttachment(usize),
    ComposeFilesDropped(DraggedFiles),
    ComposeFileTransfer(String),
    ComposeFileTransferResolved(Result<Vec<String>, String>),
    ComposeDragEnter,
    ComposeDragLeave,
    ComposeSend,
    ComposeCancel,
    SendComplete(Result<(), String>),

    ImapEvent(AccountId, ImapWatchEvent),

    // Search
    SearchActivate,
    SearchQueryChanged(String),
    SearchExecute,
    SearchResultsLoaded(Result<Vec<MessageSummary>, String>),
    SearchClear,

    // Message-to-folder drag
    DragMessageToFolder {
        envelope_hash: u64,
        source_mailbox: u64,
        dest_mailbox: u64,
    },
    FolderDragEnter(usize),
    FolderDragLeave,

    PaneResized(pane_grid::ResizeEvent),

    /// Auto-mark-read: fires 5s after a message is displayed
    AutoMarkRead(u64),

    ForceReconnect(AccountId),
    Refresh,
    Noop,

    // Account management
    AccountAdd,
    AccountEdit(AccountId),
    AccountRemove(AccountId),
    ToggleAccountCollapse(usize),

    // Setup dialog messages
    SetupLabelChanged(String),
    SetupServerChanged(String),
    SetupPortChanged(String),
    SetupUsernameChanged(String),
    SetupPasswordChanged(String),
    SetupStarttlsToggled(bool),
    SetupPasswordVisibilityToggled,
    SetupEmailAddressesChanged(String),
    SetupSmtpServerChanged(String),
    SetupSmtpPortChanged(String),
    SetupSmtpUsernameChanged(String),
    SetupSmtpPasswordChanged(String),
    SetupSmtpStarttlsToggled(bool),
    SetupSubmit,
    SetupCancel,
}

#[derive(Debug, Clone)]
pub enum ImapWatchEvent {
    NewMessage {
        mailbox_hash: u64,
        subject: String,
        from: String,
    },
    MessageRemoved {
        mailbox_hash: u64,
        envelope_hash: u64,
    },
    FlagsChanged {
        mailbox_hash: u64,
        envelope_hash: u64,
        flags: u8,
    },
    Rescan,
    WatchError(String),
    WatchEnded,
}
