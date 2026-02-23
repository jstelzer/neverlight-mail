use cosmic::iced::Length;
use cosmic::widget;
use cosmic::Element;

use crate::app::Message;
use crate::core::models::MessageSummary;

/// Render the message list for the selected folder.
pub fn view<'a>(messages: &'a [MessageSummary], selected: Option<usize>) -> Element<'a, Message> {
    let mut col = widget::column().spacing(2).padding(8);

    if messages.is_empty() {
        col = col.push(widget::text::body("No messages"));
    } else {
        for (i, msg) in messages.iter().enumerate() {
            let _is_selected = selected == Some(i);

            let subject = widget::text::body(&msg.subject);
            let meta = widget::text::caption(format!("{} — {}", msg.from, msg.date));

            let row_content = widget::column().push(subject).push(meta).spacing(2);

            let btn = widget::button::custom(row_content)
                .on_press(Message::SelectMessage(i))
                .width(Length::Fill);

            col = col.push(btn);
        }
    }

    widget::scrollable(col).height(Length::Fill).into()
}
