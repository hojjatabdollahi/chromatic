// SPDX-License-Identifier: MPL-2.0

//! Documents page view for the Chromatic application.

use crate::app::{AppModel, ConnectionStatus, Message};
use crate::fl;
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::widget::{self, icon};

use super::widgets::{connection_status_badge, document_card};

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

    // Build the main page content
    let main_content = widget::column::with_capacity(3)
        .push(header)
        .push(toolbar)
        .push(content)
        .spacing(space_m)
        .width(Length::Fill)
        .height(Length::Fill);

    // Check for delete confirmation dialog
    if let Some(ref document) = app.delete_document_target {
        let dialog: Element<'_, Message> = widget::dialog()
            .title(fl!("delete-document"))
            .body(format!(
                "{}: '{}'",
                fl!("confirm-delete-document"),
                document.id
            ))
            .primary_action(
                widget::button::destructive(fl!("delete")).on_press(Message::ConfirmDeleteDocument),
            )
            .secondary_action(
                widget::button::standard(fl!("cancel")).on_press(Message::CancelDeleteDocument),
            )
            .into();

        return widget::popover(main_content)
            .modal(true)
            .popup(dialog)
            .into();
    }

    main_content.into()
}
