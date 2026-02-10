// SPDX-License-Identifier: MPL-2.0

//! Shared widgets for the Chromatic application pages.

use crate::api::{Collection, Document};
use crate::app::{ConnectionStatus, Message, Notification, NotificationLevel};
use crate::fl;
use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::widget::{self, icon};

/// Connection status badge widget
pub fn connection_status_badge(status: &ConnectionStatus) -> Element<'static, Message> {
    match status {
        ConnectionStatus::Disconnected => {
            widget::button::custom(widget::text::body(fl!("disconnected")))
                .class(cosmic::theme::Button::Standard)
                .into()
        }
        ConnectionStatus::Connecting => {
            widget::button::custom(widget::text::body(fl!("connecting")))
                .class(cosmic::theme::Button::Standard)
                .into()
        }
        ConnectionStatus::Connected => {
            // Green checkmark for connected status
            widget::button::custom(
                widget::row::with_capacity(2)
                    .push(icon::from_name("object-select-symbolic").size(16))
                    .push(widget::text::body(fl!("connected")))
                    .spacing(4)
                    .align_y(Alignment::Center),
            )
            .class(cosmic::theme::Button::Suggested)
            .into()
        }
        ConnectionStatus::Error(_) => widget::button::custom(widget::text::body(fl!("error")))
            .class(cosmic::theme::Button::Destructive)
            .into(),
    }
}

/// Notification toast widget
pub fn notification_toast(notification: &Notification) -> Element<'_, Message> {
    let id = notification.id;

    // Choose style based on level
    let container_style = match notification.level {
        NotificationLevel::Info => cosmic::style::Container::Card,
        NotificationLevel::Success => cosmic::style::Container::Card,
        NotificationLevel::Warning => cosmic::style::Container::Card,
        NotificationLevel::Error => cosmic::style::Container::Card,
    };

    // Icon based on level
    let level_icon = match notification.level {
        NotificationLevel::Info => icon::from_name("dialog-information-symbolic").size(20),
        NotificationLevel::Success => icon::from_name("object-select-symbolic").size(20),
        NotificationLevel::Warning => icon::from_name("dialog-warning-symbolic").size(20),
        NotificationLevel::Error => icon::from_name("dialog-error-symbolic").size(20),
    };

    // Content row with icon, text, and buttons
    let content = widget::row::with_capacity(4)
        .push(level_icon)
        .push(
            widget::column::with_capacity(2)
                .push(widget::text::body(&notification.title))
                .push_maybe(if notification.message.is_empty() {
                    None
                } else {
                    Some(widget::text::caption(&notification.message))
                })
                .spacing(2)
                .width(Length::Fill),
        )
        .push(
            widget::button::icon(icon::from_name("edit-copy-symbolic"))
                .on_press(Message::CopyNotification(id))
                .class(cosmic::theme::Button::Standard),
        )
        .push(
            widget::button::icon(icon::from_name("window-close-symbolic"))
                .on_press(Message::DismissNotification(id))
                .class(cosmic::theme::Button::Standard),
        )
        .spacing(8)
        .align_y(Alignment::Center);

    widget::container(content)
        .padding(12)
        .width(Length::Fixed(400.0))
        .class(container_style)
        .into()
}

/// Document details view for the context drawer
pub fn document_details_view(document: Option<&Document>) -> Element<'_, Message> {
    let space_s = cosmic::theme::spacing().space_s;

    let Some(doc) = document else {
        return widget::text::body(fl!("no-document-selected")).into();
    };

    let mut content = widget::column::with_capacity(10).spacing(space_s);

    // Document ID section
    content = content.push(widget::text::title4(fl!("document-id")));
    content = content.push(
        widget::container(widget::text::body(doc.id.clone()))
            .padding(space_s)
            .width(Length::Fill)
            .class(cosmic::style::Container::Card),
    );

    // Document content section
    content = content.push(widget::text::title4(fl!("document-content")));
    let doc_content = doc
        .document
        .clone()
        .unwrap_or_else(|| "[No content]".to_string());
    content = content.push(
        widget::container(
            widget::scrollable(widget::text::body(doc_content)).height(Length::Fixed(200.0)),
        )
        .padding(space_s)
        .width(Length::Fill)
        .class(cosmic::style::Container::Card),
    );

    // Metadata section
    if let Some(ref metadata) = doc.metadata {
        if !metadata.is_empty() {
            content = content.push(widget::text::title4(fl!("metadata")));

            let mut metadata_column = widget::column::with_capacity(metadata.len()).spacing(4);
            for (key, value) in metadata {
                let row = widget::row::with_capacity(2)
                    .push(widget::text::body(format!("{}:", key)).width(Length::Fixed(120.0)))
                    .push(widget::text::caption(value.to_string()))
                    .spacing(8);
                metadata_column = metadata_column.push(row);
            }

            content = content.push(
                widget::container(metadata_column)
                    .padding(space_s)
                    .width(Length::Fill)
                    .class(cosmic::style::Container::Card),
            );
        }
    }

    widget::scrollable(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Collection card widget with actions (show documents, delete)
pub fn collection_card(collection: &Collection, space_s: u16) -> Element<'_, Message> {
    let collection_for_select = collection.clone();
    let collection_for_delete = collection.clone();

    // Collection info
    let info_column = widget::column::with_capacity(2)
        .push(widget::text::title4(&collection.name))
        .push(widget::text::caption(format!("ID: {}", collection.id)))
        .spacing(4)
        .width(Length::Fill);

    // Action buttons
    let actions = widget::row::with_capacity(2)
        .push(
            widget::button::icon(icon::from_name("folder-open-symbolic"))
                .on_press(Message::SelectCollection(collection_for_select))
                .class(cosmic::theme::Button::Standard),
        )
        .push(
            widget::button::icon(icon::from_name("user-trash-symbolic"))
                .on_press(Message::RequestDeleteCollection(collection_for_delete))
                .class(cosmic::theme::Button::Destructive),
        )
        .spacing(4)
        .align_y(Alignment::Center);

    // Main row with info and actions
    let card_content = widget::row::with_capacity(2)
        .push(info_column)
        .push(actions)
        .spacing(space_s)
        .align_y(Alignment::Center);

    widget::container(card_content)
        .padding(space_s)
        .width(Length::Fill)
        .class(cosmic::style::Container::Card)
        .into()
}

/// Document card widget with actions (show details, delete)
pub fn document_card(doc: &Document, space_s: u16) -> Element<'_, Message> {
    let doc_for_details = doc.clone();
    let doc_for_delete = doc.clone();

    let doc_content = doc.document.as_deref().unwrap_or("[No content]");

    // Create a preview with better truncation
    let preview = if doc_content.len() > 200 {
        format!("{}...", &doc_content[..200])
    } else {
        doc_content.to_string()
    };

    // Build metadata display (limited to 3 items)
    let metadata_items: Vec<Element<'_, Message>> = doc
        .metadata
        .as_ref()
        .map(|m| {
            m.iter()
                .take(3)
                .map(|(k, v)| {
                    widget::row::with_capacity(2)
                        .push(
                            widget::container(widget::text::caption(format!("{}:", k)))
                                .width(Length::Fixed(80.0)),
                        )
                        .push(widget::text::caption(v.to_string()))
                        .spacing(4)
                        .into()
                })
                .collect()
        })
        .unwrap_or_default();

    // Document ID badge
    let id_badge = widget::container(widget::text::caption(&doc.id))
        .padding([2, 8])
        .class(cosmic::style::Container::Primary);

    // Action buttons
    let actions = widget::row::with_capacity(2)
        .push(
            widget::button::icon(icon::from_name("document-properties-symbolic"))
                .on_press(Message::ShowDocumentDetails(doc_for_details))
                .class(cosmic::theme::Button::Standard),
        )
        .push(
            widget::button::icon(icon::from_name("user-trash-symbolic"))
                .on_press(Message::RequestDeleteDocument(doc_for_delete))
                .class(cosmic::theme::Button::Destructive),
        )
        .spacing(4)
        .align_y(Alignment::Center);

    // Header row with ID and actions
    let header = widget::row::with_capacity(2)
        .push(id_badge)
        .push(widget::Space::with_width(Length::Fill))
        .push(actions)
        .align_y(Alignment::Center);

    let mut card_content = widget::column::with_capacity(4).spacing(space_s);
    card_content = card_content.push(header);

    // Document content preview
    card_content =
        card_content.push(widget::container(widget::text::body(preview)).padding([4, 0]));

    // Metadata section (if any)
    if !metadata_items.is_empty() {
        card_content = card_content
            .push(widget::text::caption(fl!("metadata")).class(cosmic::style::Text::Accent));
        card_content = card_content.push(
            widget::container(widget::column::with_children(metadata_items).spacing(2))
                .padding([space_s, 0, 0, 0]),
        );
    }

    widget::container(card_content)
        .padding(space_s)
        .width(Length::Fill)
        .class(cosmic::style::Container::Card)
        .into()
}
