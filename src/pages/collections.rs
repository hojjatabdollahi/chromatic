// SPDX-License-Identifier: MPL-2.0

//! Collections page view for the Chromatic application.

use crate::app::{AppModel, ConnectionStatus, Message};
use crate::fl;
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::widget;

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
        let mut list_column = widget::column::with_capacity(app.collections.len());

        for collection in &app.collections {
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
