// SPDX-License-Identifier: MPL-2.0

//! Dashboard page view for the Chromatic application.

use crate::app::{AppModel, ConnectionStatus, Message};
use crate::fl;
use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::widget;

use super::widgets::connection_status_badge;

/// View for the Dashboard page
pub fn view(app: &AppModel, _space_s: u16, space_m: u16) -> Element<'_, Message> {
    let header = widget::row::with_capacity(2)
        .push(widget::text::title1(fl!("dashboard")))
        .push(connection_status_badge(&app.connection_status))
        .align_y(Alignment::Center)
        .spacing(space_m);

    let refresh_button =
        widget::button::standard(fl!("refresh")).on_press(Message::FetchServerInfo);

    // Stats cards
    let version_card = stat_card(
        fl!("server-version"),
        app.server_info
            .as_ref()
            .map(|i| i.version.clone())
            .unwrap_or_else(|| "-".to_string()),
    );

    let heartbeat_card = stat_card(
        fl!("heartbeat"),
        app.server_info
            .as_ref()
            .map(|i| {
                // Convert nanoseconds to a readable format
                let secs = i.heartbeat_ns / 1_000_000_000;
                format!("{} s", secs)
            })
            .unwrap_or_else(|| "-".to_string()),
    );

    let active = app.config.active_config();
    let tenant_card = stat_card(fl!("current-tenant"), active.tenant.clone());

    let database_card = stat_card(fl!("current-database"), active.database.clone());

    let collections_card = stat_card(fl!("collection-count"), app.collections.len().to_string());

    let api_version_card = stat_card(
        fl!("api-version"),
        app.server_info
            .as_ref()
            .map(|i| i.api_version.clone())
            .unwrap_or_else(|| "-".to_string()),
    );

    let stats_row1 = widget::row::with_capacity(4)
        .push(version_card)
        .push(api_version_card)
        .push(heartbeat_card)
        .push(collections_card)
        .spacing(space_m);

    let stats_row2 = widget::row::with_capacity(2)
        .push(tenant_card)
        .push(database_card)
        .spacing(space_m);

    let content: Element<'_, Message> = match &app.connection_status {
        ConnectionStatus::Disconnected | ConnectionStatus::Error(_) => {
            widget::column::with_capacity(2)
                .push(
                    widget::container(widget::text::body(fl!("dashboard-connect-hint")))
                        .padding(space_m)
                        .width(Length::Fill)
                        .class(cosmic::style::Container::Card),
                )
                .push(refresh_button)
                .spacing(space_m)
                .into()
        }
        _ => widget::column::with_capacity(3)
            .push(refresh_button)
            .push(stats_row1)
            .push(stats_row2)
            .spacing(space_m)
            .into(),
    };

    widget::column::with_capacity(2)
        .push(header)
        .push(content)
        .spacing(space_m)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Helper to create a stat card widget
fn stat_card(label: String, value: String) -> Element<'static, Message> {
    widget::container(
        widget::column::with_capacity(2)
            .push(widget::text::caption(label))
            .push(widget::text::title3(value))
            .spacing(4),
    )
    .padding(cosmic::theme::spacing().space_s)
    .width(Length::FillPortion(1))
    .class(cosmic::style::Container::Card)
    .into()
}
