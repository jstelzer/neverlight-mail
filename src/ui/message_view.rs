use cosmic::iced::Length;
use cosmic::widget;
use cosmic::Element;

use crate::app::Message;

/// Render the message preview pane.
pub fn view<'a>(body: &'a str) -> Element<'a, Message> {
    let content = if body.is_empty() {
        widget::text::body("Select a message to read")
    } else {
        widget::text::body(body)
    };

    widget::scrollable(
        widget::container(content)
            .padding(16)
            .width(Length::Fill),
    )
    .height(Length::Fill)
    .into()
}
