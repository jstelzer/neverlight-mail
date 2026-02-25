use cosmic::iced::Length;
use cosmic::widget;
use cosmic::Element;

use crate::app::{ConnectionState, Message};
use crate::core::models::{DraggedMessage, Folder};

/// Render the folder sidebar.
pub fn view<'a>(
    folders: &[Folder],
    _selected: Option<usize>,
    conn_state: &'a ConnectionState,
    drag_target: Option<usize>,
) -> Element<'a, Message> {
    let mut col = widget::column().spacing(4).padding(8);

    col = col.push(
        widget::button::suggested("Compose")
            .on_press(Message::ComposeNew)
            .width(Length::Fill),
    );
    col = col.push(widget::vertical_space().height(8));

    if folders.is_empty() {
        col = col.push(widget::text::body("No folders"));
    } else {
        for (i, folder) in folders.iter().enumerate() {
            let label = if folder.unread_count > 0 {
                format!("{} ({})", folder.name, folder.unread_count)
            } else {
                folder.name.clone()
            };

            let is_drag_target = drag_target == Some(i);
            let mut btn = widget::button::text(label)
                .on_press(Message::SelectFolder(i))
                .width(Length::Fill);

            if is_drag_target {
                btn = btn.class(cosmic::theme::Button::Suggested);
            }

            let mailbox_hash = folder.mailbox_hash;
            let dest = widget::dnd_destination::dnd_destination_for_data::<DraggedMessage, _>(
                btn,
                move |data, _action| match data {
                    Some(msg) => Message::DragMessageToFolder {
                        envelope_hash: msg.envelope_hash,
                        source_mailbox: msg.source_mailbox,
                        dest_mailbox: mailbox_hash,
                    },
                    None => Message::Noop,
                },
            )
            .on_enter(move |_x, _y, _mimes| Message::FolderDragEnter(i))
            .on_leave(|| Message::FolderDragLeave);

            col = col.push(dest);
        }
    }

    let scrollable_folders = widget::scrollable(col).height(Length::Fill);

    let status_pill = status_pill_view(conn_state);

    widget::column()
        .push(scrollable_folders)
        .push(status_pill)
        .height(Length::Fill)
        .into()
}

fn status_pill_view(conn_state: &ConnectionState) -> Element<'_, Message> {
    let label = match conn_state {
        ConnectionState::Connected => "● Connected".to_string(),
        ConnectionState::Connecting => "◌ Connecting...".to_string(),
        ConnectionState::Syncing => "◌ Syncing...".to_string(),
        ConnectionState::Error(msg) => format!("● Offline — {}", msg),
        ConnectionState::Disconnected => "○ Disconnected".to_string(),
    };

    let clickable = matches!(
        conn_state,
        ConnectionState::Connected | ConnectionState::Error(_) | ConnectionState::Disconnected
    );

    let pill = widget::container(widget::text::caption(label)).padding([6, 8]);

    if clickable {
        widget::button::custom(pill)
            .on_press(Message::ForceReconnect)
            .class(cosmic::theme::Button::Text)
            .width(Length::Fill)
            .into()
    } else {
        pill.width(Length::Fill).into()
    }
}
