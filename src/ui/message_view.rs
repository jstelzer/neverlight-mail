use cosmic::iced::{ContentFit, Length};
use cosmic::widget;
use cosmic::widget::{image, markdown, text_editor};
use cosmic::Element;

use crate::app::{ConversationEntry, Message};
use neverlight_mail_core::models::{AttachmentData, MessageSummary};

/// Render the message preview pane with an action toolbar when a message is selected.
pub fn view<'a>(
    markdown_items: &'a [markdown::Item],
    preview_editor: &'a text_editor::Content,
    selectable: bool,
    selected: Option<(usize, &'a MessageSummary)>,
    attachments: &[AttachmentData],
    image_handles: &[Option<image::Handle>],
    conversation: &'a [ConversationEntry],
    conversation_editors: &'a [text_editor::Content],
    active_email_id: Option<&'a str>,
) -> Element<'a, Message> {
    if !conversation.is_empty() {
        return conversation_view(
            conversation,
            conversation_editors,
            selectable,
            active_email_id,
            selected,
        );
    }

    let has_body = if selectable {
        !preview_editor.text().trim().is_empty()
    } else {
        !markdown_items.is_empty()
    };

    if !has_body && attachments.is_empty() {
        return widget::container(widget::text::body("Select a message to read"))
            .padding(16)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    }

    let mut col = widget::column().spacing(0);

    if let Some((index, msg)) = selected {
        col = col.push(toolbar(index, msg, selectable));
        col = col.push(
            widget::container(message_header(msg))
                .padding([4, 16])
                .width(Length::Fill)
                .class(cosmic::style::Container::Card),
        );
    }

    if has_body {
        col = col.push(body_widget(markdown_items, preview_editor, selectable));
    }

    if !attachments.is_empty() {
        col = col.push(attachments_section(attachments, image_handles, None));
    }

    widget::scrollable(col)
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}

/// Render a body region — markdown (rich) or text_editor (selectable).
fn body_widget<'a>(
    markdown_items: &'a [markdown::Item],
    editor: &'a text_editor::Content,
    selectable: bool,
) -> Element<'a, Message> {
    if selectable {
        widget::container(
            widget::text_editor(editor).on_action(Message::PreviewBodyAction),
        )
        .padding(16)
        .width(Length::Fill)
        .into()
    } else {
        let md = markdown::view(
            markdown_items,
            markdown::Settings::default(),
            markdown::Style::from_palette(cosmic::iced::Theme::Dark.palette()),
        )
        .map(Message::LinkClicked);

        widget::container(md).padding(16).width(Length::Fill).into()
    }
}

fn conversation_view<'a>(
    conversation: &'a [ConversationEntry],
    conversation_editors: &'a [text_editor::Content],
    selectable: bool,
    active_email_id: Option<&'a str>,
    selected: Option<(usize, &'a MessageSummary)>,
) -> Element<'a, Message> {
    let mut col = widget::column().spacing(0);

    // Toolbar for the active message
    if let Some((index, msg)) = selected {
        col = col.push(toolbar(index, msg, selectable));
    }

    // Stacked message cards
    for (entry_idx, entry) in conversation.iter().enumerate() {
        let is_active = active_email_id == Some(entry.email_id.as_str());

        let mut card_col = widget::column().spacing(4);

        // Header
        card_col = card_col.push(message_header(&entry.summary));

        // Body
        if entry.loaded {
            let has_content = if selectable {
                conversation_editors
                    .get(entry_idx)
                    .map_or(false, |e| !e.text().trim().is_empty())
            } else {
                !entry.markdown_items.is_empty()
            };

            if has_content {
                if selectable {
                    if let Some(editor) = conversation_editors.get(entry_idx) {
                        card_col = card_col.push(
                            widget::container(
                                widget::text_editor(editor).on_action(move |action| {
                                    Message::ConversationBodyAction {
                                        index: entry_idx,
                                        action,
                                    }
                                }),
                            )
                            .padding([8, 0])
                            .width(Length::Fill),
                        );
                    }
                } else {
                    let md = markdown::view(
                        &entry.markdown_items,
                        markdown::Settings::default(),
                        markdown::Style::from_palette(
                            cosmic::iced::Theme::Dark.palette(),
                        ),
                    )
                    .map(Message::LinkClicked);
                    card_col = card_col.push(
                        widget::container(md)
                            .padding([8, 0])
                            .width(Length::Fill),
                    );
                }
            }

            if !entry.attachments.is_empty() {
                card_col = card_col.push(attachments_section(
                    &entry.attachments,
                    &entry.image_handles,
                    Some(&entry.email_id),
                ));
            }
        } else {
            card_col = card_col.push(
                widget::text::body("Loading...")
                    .font(cosmic::iced::Font {
                        style: cosmic::iced::font::Style::Italic,
                        ..Default::default()
                    }),
            );
        }

        let container_class = if entry.is_sent {
            cosmic::style::Container::Primary
        } else {
            cosmic::style::Container::Card
        };

        let border_width = if is_active { 2.0 } else { 0.0 };

        let card = widget::container(card_col)
            .padding([8, 16])
            .width(Length::Fill)
            .class(container_class);

        // Wrap in a container with accent border when active
        let card_with_border: Element<'a, Message> = if is_active {
            widget::container(card)
                .width(Length::Fill)
                .style(move |theme: &cosmic::Theme| {
                    let cosmic = theme.cosmic();
                    cosmic::iced_widget::container::Style {
                        border: cosmic::iced::Border {
                            color: cosmic.accent_color().into(),
                            width: border_width,
                            radius: 8.0.into(),
                        },
                        ..Default::default()
                    }
                })
                .into()
        } else {
            card.into()
        };

        let email_id = entry.email_id.clone();
        let clickable = widget::mouse_area(card_with_border)
            .on_press(Message::SetActiveConversation(email_id));

        col = col.push(
            widget::container(clickable)
                .padding([4, 0])
                .width(Length::Fill),
        );
    }

    widget::scrollable(col)
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}

fn toolbar<'a>(index: usize, msg: &MessageSummary, selectable: bool) -> Element<'a, Message> {
    let star_label = if msg.is_starred {
        "\u{2605}"
    } else {
        "\u{2606}"
    };
    let read_label = if msg.is_read {
        "Mark unread"
    } else {
        "Mark read"
    };
    let select_label = if selectable { "Rich text" } else { "Select text" };

    let toolbar = widget::row()
        .spacing(8)
        .push(widget::button::text("Reply").on_press(Message::ComposeReply))
        .push(widget::button::text("Forward").on_press(Message::ComposeForward))
        .push(widget::button::text(star_label).on_press(Message::ToggleStar(index)))
        .push(widget::button::text(read_label).on_press(Message::ToggleRead(index)))
        .push(widget::button::text("Archive").on_press(Message::Archive(index)))
        .push(widget::button::text("Copy").on_press(Message::CopyBody))
        .push(
            widget::button::text(select_label).on_press(Message::ToggleSelectableView),
        )
        .push(widget::button::destructive("Trash").on_press(Message::Delete(index)));

    widget::container(toolbar)
        .padding([8, 16])
        .width(Length::Fill)
        .into()
}

fn header_row<'a>(label: &'a str, value: &'a str) -> Element<'a, Message> {
    widget::row()
        .spacing(8)
        .push(
            widget::text::body(label)
                .width(Length::Fixed(80.0))
                .font(cosmic::iced::Font {
                    weight: cosmic::iced::font::Weight::Bold,
                    ..Default::default()
                }),
        )
        .push(widget::text::body(value).width(Length::Fill))
        .into()
}

fn message_header<'a>(msg: &'a MessageSummary) -> Element<'a, Message> {
    let mut col = widget::column().spacing(4);
    col = col.push(header_row("From:", &msg.from));
    if !msg.to.is_empty() {
        col = col.push(header_row("To:", &msg.to));
    }
    col = col.push(header_row("Subject:", &msg.subject));
    col = col.push(header_row("Date:", &msg.date));
    if let Some(ref reply_to) = msg.reply_to {
        col = col.push(header_row("Reply-To:", reply_to));
    }
    col.into()
}

/// Render attachments. If `conversation_email_id` is Some, use SaveConversationAttachment.
fn attachments_section<'a>(
    attachments: &[AttachmentData],
    image_handles: &[Option<image::Handle>],
    conversation_email_id: Option<&str>,
) -> Element<'a, Message> {
    let mut att_col = widget::column().spacing(8);

    att_col = att_col.push(widget::text::heading(format!(
        "Attachments ({})",
        attachments.len()
    )));

    for (i, att) in attachments.iter().enumerate() {
        let mut card = widget::column().spacing(4);

        // Image preview
        if let Some(Some(handle)) = image_handles.get(i) {
            card = card.push(
                widget::Image::new(handle.clone())
                    .content_fit(ContentFit::Contain)
                    .width(Length::Fill),
            );
        }

        // Filename, size, save button
        let size_str = human_size(att.data.len());
        let save_msg = if let Some(eid) = conversation_email_id {
            Message::SaveConversationAttachment {
                email_id: eid.to_string(),
                index: i,
            }
        } else {
            Message::SaveAttachment(i)
        };
        let info = widget::row()
            .spacing(8)
            .align_y(cosmic::iced::Alignment::Center)
            .push(
                widget::text::body(format!("{} ({})", att.filename, size_str))
                    .width(Length::Fill),
            )
            .push(widget::button::suggested("Save").on_press(save_msg));

        card = card.push(info);

        att_col = att_col.push(
            widget::container(card)
                .padding(8)
                .width(Length::Fill)
                .class(cosmic::style::Container::Card),
        );
    }

    widget::container(att_col)
        .padding([8, 16])
        .width(Length::Fill)
        .into()
}

fn human_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
