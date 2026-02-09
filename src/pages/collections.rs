// SPDX-License-Identifier: MPL-2.0

//! Collections page view for the Chromatic application.

use crate::app::{AppModel, ConnectionStatus, Message};
use crate::fl;
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::widget::{self, icon};

use super::widgets::connection_status_badge;

/// View for the Collections page
pub fn view(app: &AppModel, space_s: u16, space_m: u16) -> Element<'_, Message> {
    let header = widget::row::with_capacity(2)
        .push(widget::text::title1(fl!("collections")))
        .push(connection_status_badge(&app.connection_status))
        .align_y(Alignment::Center)
        .spacing(space_m);

    let refresh_button =
        widget::button::standard(fl!("refresh")).on_press(Message::FetchCollections);

    let toolbar = widget::row::with_capacity(1)
        .push(refresh_button)
        .spacing(space_s);

    let content: Element<'_, Message> = if app.collections.is_empty() {
        let empty_message = match &app.connection_status {
            ConnectionStatus::Disconnected => fl!("not-connected"),
            ConnectionStatus::Connecting => fl!("connecting"),
            ConnectionStatus::Connected => fl!("no-collections"),
            ConnectionStatus::Error(e) => e.clone(),
        };

        widget::container(widget::text::body(empty_message))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .into()
    } else {
        // Calculate pagination
        let total_items = app.collections.len();
        let total_pages = (total_items + app.items_per_page - 1) / app.items_per_page;
        let start_idx = app.collections_page * app.items_per_page;
        let end_idx = (start_idx + app.items_per_page).min(total_items);
        let page_items = &app.collections[start_idx..end_idx];

        let mut list_column = widget::column::with_capacity(page_items.len());

        for collection in page_items {
            let collection_clone = collection.clone();
            let item = widget::mouse_area(
                widget::container(
                    widget::column::with_capacity(2)
                        .push(widget::text::title4(&collection.name))
                        .push(widget::text::caption(format!("ID: {}", collection.id)))
                        .spacing(4),
                )
                .padding(space_s)
                .width(Length::Fill)
                .class(cosmic::style::Container::Card),
            )
            .on_press(Message::SelectCollection(collection_clone));

            list_column = list_column.push(item);
        }

        // Pagination controls
        let mut pagination_row = widget::row::with_capacity(5)
            .spacing(space_s)
            .align_y(Alignment::Center);

        // Previous button
        let prev_button = if app.collections_page > 0 {
            widget::button::icon(icon::from_name("go-previous-symbolic"))
                .on_press(Message::CollectionsPrevPage)
        } else {
            widget::button::icon(icon::from_name("go-previous-symbolic"))
        };
        pagination_row = pagination_row.push(prev_button);

        // Page info
        let page_info = widget::text::body(format!(
            "{} {} / {}",
            fl!("page"),
            app.collections_page + 1,
            total_pages.max(1)
        ));
        pagination_row = pagination_row.push(page_info);

        // Next button
        let next_button = if app.collections_page + 1 < total_pages {
            widget::button::icon(icon::from_name("go-next-symbolic"))
                .on_press(Message::CollectionsNextPage)
        } else {
            widget::button::icon(icon::from_name("go-next-symbolic"))
        };
        pagination_row = pagination_row.push(next_button);

        // Total items count
        pagination_row = pagination_row.push(widget::text::caption(format!(
            "({} {})",
            total_items,
            fl!("items-total")
        )));

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
