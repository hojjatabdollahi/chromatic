// SPDX-License-Identifier: MPL-2.0

//! Documents page view for the Chromatic application.

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

    let doc_count = widget::text::body(format!(
        "{} {}",
        app.documents.len(),
        fl!("documents-count")
    ));

    let toolbar = widget::row::with_capacity(2)
        .push(refresh_button)
        .push(doc_count)
        .spacing(space_s)
        .align_y(Alignment::Center);

    let content: Element<'_, Message> = if app.documents.is_empty() {
        let empty_message = match &app.connection_status {
            ConnectionStatus::Disconnected => fl!("not-connected"),
            ConnectionStatus::Connecting => fl!("loading-documents"),
            ConnectionStatus::Connected => fl!("no-documents"),
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
            let doc_content = doc.document.as_deref().unwrap_or("[No content]");
            let preview = if doc_content.len() > 200 {
                format!("{}...", &doc_content[..200])
            } else {
                doc_content.to_string()
            };

            let metadata_str = doc
                .metadata
                .as_ref()
                .map(|m| {
                    m.iter()
                        .map(|(k, v)| format!("{}: {}", k, v))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();

            let mut item_content = widget::column::with_capacity(3)
                .push(widget::text::title4(&doc.id))
                .push(widget::text::body(preview))
                .spacing(4);

            if !metadata_str.is_empty() {
                item_content = item_content.push(widget::text::caption(metadata_str));
            }

            let item = widget::container(item_content)
                .padding(space_s)
                .width(Length::Fill)
                .class(cosmic::style::Container::Card);

            list_column = list_column.push(item);
        }

        widget::scrollable(list_column.spacing(space_s))
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
