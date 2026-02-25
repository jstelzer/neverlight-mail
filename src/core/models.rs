use std::borrow::Cow;

use cosmic::iced::clipboard::mime::{AllowedMimeTypes, AsMimeTypes};
use serde::{Deserialize, Serialize};

/// A mail folder (IMAP mailbox).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    pub name: String,
    pub path: String,
    pub unread_count: u32,
    pub total_count: u32,
    pub mailbox_hash: u64,
}

/// Summary of a message for the list view (no body).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageSummary {
    pub uid: u64,
    pub subject: String,
    pub from: String,
    pub date: String,
    pub is_read: bool,
    pub is_starred: bool,
    pub has_attachments: bool,
    pub thread_id: Option<u64>,
    pub envelope_hash: u64,
    pub timestamp: i64,
    pub mailbox_hash: u64,
    pub message_id: String,
    pub in_reply_to: Option<String>,
    pub reply_to: Option<String>,
    pub thread_depth: u32,
}

/// Decoded attachment data for display and saving.
#[derive(Debug, Clone)]
pub struct AttachmentData {
    pub filename: String,
    pub mime_type: String,
    pub data: Vec<u8>,
}

impl AttachmentData {
    pub fn is_image(&self) -> bool {
        self.mime_type
            .to_ascii_lowercase()
            .starts_with("image/")
    }
}

// ---------------------------------------------------------------------------
// Drag-and-drop data types
// ---------------------------------------------------------------------------

/// External file drop data (text/uri-list from file managers).
#[derive(Debug, Clone)]
pub struct DraggedFiles(pub String);

impl AllowedMimeTypes for DraggedFiles {
    fn allowed() -> Cow<'static, [String]> {
        Cow::Owned(vec!["text/uri-list".to_string()])
    }
}

impl TryFrom<(Vec<u8>, String)> for DraggedFiles {
    type Error = String;
    fn try_from((bytes, _mime): (Vec<u8>, String)) -> Result<Self, Self::Error> {
        String::from_utf8(bytes)
            .map(DraggedFiles)
            .map_err(|e| e.to_string())
    }
}

/// Internal message drag data for message-to-folder moves.
#[derive(Debug, Clone)]
pub struct DraggedMessage {
    pub envelope_hash: u64,
    pub source_mailbox: u64,
}

const NEVERMAIL_MIME: &str = "application/x-nevermail-message";

impl AsMimeTypes for DraggedMessage {
    fn available(&self) -> Cow<'static, [String]> {
        Cow::Owned(vec![NEVERMAIL_MIME.to_string()])
    }

    fn as_bytes(&self, mime_type: &str) -> Option<Cow<'static, [u8]>> {
        if mime_type == NEVERMAIL_MIME {
            let s = format!("{}:{}", self.envelope_hash, self.source_mailbox);
            Some(Cow::Owned(s.into_bytes()))
        } else {
            None
        }
    }
}

impl AllowedMimeTypes for DraggedMessage {
    fn allowed() -> Cow<'static, [String]> {
        Cow::Owned(vec![NEVERMAIL_MIME.to_string()])
    }
}

impl TryFrom<(Vec<u8>, String)> for DraggedMessage {
    type Error = String;
    fn try_from((bytes, _mime): (Vec<u8>, String)) -> Result<Self, Self::Error> {
        let s = String::from_utf8(bytes).map_err(|e| e.to_string())?;
        let (a, b) = s.split_once(':').ok_or("missing ':' separator")?;
        Ok(DraggedMessage {
            envelope_hash: a.parse().map_err(|e: std::num::ParseIntError| e.to_string())?,
            source_mailbox: b.parse().map_err(|e: std::num::ParseIntError| e.to_string())?,
        })
    }
}

