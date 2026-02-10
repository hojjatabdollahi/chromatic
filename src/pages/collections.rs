// SPDX-License-Identifier: MPL-2.0

//! Collections page view for the Chromatic application.

use crate::app::{AppModel, ConnectionStatus, Message};
use crate::fl;
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::widget::{self, icon};

use super::widgets::{collection_card, connection_status_badge};

/// View for the Collections page
pub fn view(app: &AppModel, space_s: u16, space_m: u16) -> Element<'_, Message> {
    let header = widget::row::with_capacity(2)
        .push(widget::text::title1(fl!("collections")))
        .push(connection_status_badge(&app.connection_status))
        .align_y(Alignment::Center)
        .spacing(space_m);

    let refresh_button =
        widget::button::standard(fl!("refresh")).on_press(Message::FetchCollections);

    let new_collection_button =
        widget::button::suggested(fl!("new-collection")).on_press(Message::OpenNewCollectionDialog);

    let toolbar = widget::row::with_capacity(2)
        .push(refresh_button)
        .push(new_collection_button)
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
            list_column = list_column.push(collection_card(collection, space_s));
        }

        // Pagination controls
        let mut pagination_row = widget::row::with_capacity(5)
            .spacing(space_s)
            .align_y(Alignment::Center);

        // Previous button
        let prev_button = widget::button::icon(icon::from_name("go-previous-symbolic"))
            .class(cosmic::theme::Button::Standard)
            .on_press_maybe(if app.collections_page > 0 {
                Some(Message::CollectionsPrevPage)
            } else {
                None
            });
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
        let next_button = widget::button::icon(icon::from_name("go-next-symbolic"))
            .class(cosmic::theme::Button::Standard)
            .on_press_maybe(if app.collections_page + 1 < total_pages {
                Some(Message::CollectionsNextPage)
            } else {
                None
            });
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

    // Build the main page content
    let main_content = widget::column::with_capacity(4)
        .push(header)
        .push(toolbar)
        .push(content)
        .spacing(space_m)
        .width(Length::Fill)
        .height(Length::Fill);

    // Check for dialogs - show dialog as modal overlay
    if app.show_new_collection_dialog {
        let dialog: Element<'_, Message> = widget::dialog()
            .title(fl!("new-collection"))
            .control(
                widget::column::with_capacity(2)
                    .push(widget::text::body(fl!("collection-name")))
                    .push(
                        widget::text_input(
                            fl!("collection-name-placeholder"),
                            &app.new_collection_name,
                        )
                        .on_input(Message::NewCollectionNameChanged)
                        .width(Length::Fill),
                    )
                    .spacing(4),
            )
            .primary_action(widget::button::suggested(fl!("create")).on_press_maybe(
                if !app.new_collection_name.is_empty() {
                    Some(Message::CreateCollection)
                } else {
                    None
                },
            ))
            .secondary_action(
                widget::button::standard(fl!("cancel")).on_press(Message::CloseNewCollectionDialog),
            )
            .into();

        return widget::popover(main_content)
            .modal(true)
            .popup(dialog)
            .into();
    }

    if let Some(ref collection) = app.delete_collection_target {
        let dialog: Element<'_, Message> = widget::dialog()
            .title(fl!("delete-collection"))
            .body(format!(
                "{}: '{}'",
                fl!("confirm-delete-collection"),
                collection.name
            ))
            .primary_action(
                widget::button::destructive(fl!("delete"))
                    .on_press(Message::ConfirmDeleteCollection),
            )
            .secondary_action(
                widget::button::standard(fl!("cancel")).on_press(Message::CancelDeleteCollection),
            )
            .into();

        return widget::popover(main_content)
            .modal(true)
            .popup(dialog)
            .into();
    }

    main_content.into()
}
