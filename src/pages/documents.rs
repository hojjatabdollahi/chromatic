// SPDX-License-Identifier: MPL-2.0

//! Documents page view for the Chromatic application.

use crate::api::Document;
use crate::app::{AppModel, ConnectionStatus, Message};
use crate::fl;
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::widget::{self, icon};

use super::widgets::connection_status_badge;

/// View for the Documents page (when a collection is selected)
pub fn view(app: &AppModel, space_s: u16, space_m: u16) -> Element<'_, Message> {
    let collection_name = app
        .selected_collection
        .as_ref()
        .map(|c| c.name.as_str())
        .unwrap_or("Unknown");

    let back_button = widget::button::icon(icon::from_name("go-previous-symbolic"))
        .on_press(Message::BackToCollections);

    let header = widget::row::with_capacity(3)
        .push(back_button)
        .push(widget::text::title1(collection_name))
        .push(connection_status_badge(&app.connection_status))
        .align_y(Alignment::Center)
        .spacing(space_m);

    let refresh_button = widget::button::standard(fl!("refresh")).on_press(Message::FetchDocuments);

    // Show page info in toolbar with total count if available
    let page_info = if let Some(total) = app.documents_total {
        widget::text::body(format!(
            "{} {} | {} {} ({} {})",
            fl!("page"),
            app.documents_page + 1,
            app.documents.len(),
            fl!("documents-count"),
            total,
            fl!("items-total")
        ))
    } else {
        widget::text::body(format!(
            "{} {} | {} {}",
            fl!("page"),
            app.documents_page + 1,
            app.documents.len(),
            fl!("documents-count")
        ))
    };

    let toolbar = widget::row::with_capacity(3)
        .push(refresh_button)
        .push(page_info)
        .spacing(space_s)
        .align_y(Alignment::Center);

    let content: Element<'_, Message> = if app.documents.is_empty() {
        let empty_message = match &app.connection_status {
            ConnectionStatus::Disconnected => fl!("not-connected"),
            ConnectionStatus::Connecting => fl!("loading-documents"),
            ConnectionStatus::Connected => {
                if app.documents_page > 0 {
                    fl!("no-more-documents")
                } else {
                    fl!("no-documents")
                }
            }
            ConnectionStatus::Error(e) => e.clone(),
        };

        widget::container(widget::text::body(empty_message))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .into()
    } else {
        let mut list_column = widget::column::with_capacity(app.documents.len());

        for doc in &app.documents {
            list_column = list_column.push(document_card(doc, space_s));
        }

        // Pagination controls
        let mut pagination_row = widget::row::with_capacity(4)
            .spacing(space_s)
            .align_y(Alignment::Center);

        // Previous button
        let prev_button = widget::button::icon(icon::from_name("go-previous-symbolic"))
            .class(cosmic::theme::Button::Standard)
            .on_press_maybe(if app.documents_page > 0 {
                Some(Message::DocumentsPrevPage)
            } else {
                None
            });
        pagination_row = pagination_row.push(prev_button);

        // Page number
        pagination_row = pagination_row.push(widget::text::body(format!(
            "{} {}",
            fl!("page"),
            app.documents_page + 1
        )));

        // Next button (enabled if current page is full, indicating more might exist)
        let next_button = widget::button::icon(icon::from_name("go-next-symbolic"))
            .class(cosmic::theme::Button::Standard)
            .on_press_maybe(if app.documents.len() >= app.items_per_page {
                Some(Message::DocumentsNextPage)
            } else {
                None
            });
        pagination_row = pagination_row.push(next_button);

        widget::column::with_capacity(2)
            .push(
                widget::scrollable(list_column.spacing(space_s))
                    .width(Length::Fill)
                    .height(Length::Fill),
            )
            .push(pagination_row)
            .spacing(space_s)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    };

    widget::column::with_capacity(3)
        .push(header)
        .push(toolbar)
        .push(content)
        .spacing(space_m)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Custom document card widget with better layout
fn document_card(doc: &Document, space_s: u16) -> Element<'_, Message> {
    let doc_content = doc.document.as_deref().unwrap_or("[No content]");

    // Create a preview with better truncation
    let preview = if doc_content.len() > 300 {
        format!("{}...", &doc_content[..300])
    } else {
        doc_content.to_string()
    };

    // Build metadata display
    let metadata_items: Vec<Element<'_, Message>> = doc
        .metadata
        .as_ref()
        .map(|m| {
            m.iter()
                .take(5) // Limit to 5 metadata items
                .map(|(k, v)| {
                    widget::row::with_capacity(2)
                        .push(
                            widget::container(widget::text::caption(format!("{}:", k)))
                                .width(Length::Fixed(100.0)),
                        )
                        .push(widget::text::caption(v.to_string()))
                        .spacing(4)
                        .into()
                })
                .collect()
        })
        .unwrap_or_default();

    let mut card_content = widget::column::with_capacity(3 + metadata_items.len()).spacing(space_s);

    // Document ID header with a badge-like style
    let id_badge = widget::container(widget::text::caption(&doc.id))
        .padding([2, 8])
        .class(cosmic::style::Container::Primary);

    card_content = card_content.push(id_badge);

    // Document content preview
    card_content =
        card_content.push(widget::container(widget::text::body(preview)).padding([4, 0]));

    // Metadata section
    if !metadata_items.is_empty() {
        let metadata_section =
            widget::container(widget::column::with_children(metadata_items).spacing(2))
                .padding([space_s, 0, 0, 0]);

        card_content = card_content
            .push(widget::text::caption(fl!("metadata")).class(cosmic::style::Text::Accent));
        card_content = card_content.push(metadata_section);
    }

    widget::container(card_content)
        .padding(space_s)
        .width(Length::Fill)
        .class(cosmic::style::Container::Card)
        .into()
}
