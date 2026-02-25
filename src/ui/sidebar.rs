use cosmic::iced::Length;
use cosmic::widget;
use cosmic::Element;

use crate::app::{ConnectionState, Message};
use crate::core::models::Folder;

/// Render the folder sidebar.
pub fn view<'a>(
    folders: &[Folder],
    selected: Option<usize>,
    conn_state: &'a ConnectionState,
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
            let _is_selected = selected == Some(i);
            let label = if folder.unread_count > 0 {
                format!("{} ({})", folder.name, folder.unread_count)
            } else {
                folder.name.clone()
            };

            let btn = widget::button::text(label)
                .on_press(Message::SelectFolder(i))
                .width(Length::Fill);

            // TODO: Style differently when selected
            col = col.push(btn);
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
