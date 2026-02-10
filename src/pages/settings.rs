// SPDX-License-Identifier: MPL-2.0

//! Settings page view for the Chromatic application.

use crate::app::{AppModel, Message, SettingsStatus};
use crate::fl;
use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::widget::{self, icon};

/// View for the Settings page
pub fn view(app: &AppModel, space_s: u16, space_m: u16) -> Element<'_, Message> {
    let header = widget::text::title1(fl!("settings"));

    // Server selection - dropdown with add button
    let server_selector = widget::row::with_capacity(2)
        .push(
            widget::dropdown(&app.server_names, Some(app.config.active_server), |idx| {
                Message::SelectServer(idx)
            })
            .width(Length::Fixed(200.0)),
        )
        .push(
            widget::button::icon(icon::from_name("list-add-symbolic"))
                .on_press(Message::AddNewServer),
        )
        .spacing(space_s)
        .align_y(Alignment::Center);

    let servers_section = cosmic::widget::settings::section()
        .title(fl!("servers"))
        .add(
            cosmic::widget::settings::item::builder(fl!("saved-servers"))
                .description(fl!("saved-servers-description"))
                .flex_control(server_selector),
        );

    // Clone data for dropdown closures
    let tenants_for_dropdown = app.available_tenants.clone();
    let databases_for_dropdown = app.available_databases.clone();

    // Server configuration section
    let mut server_section = cosmic::widget::settings::section()
        .title(fl!("server-config"))
        .add(
            cosmic::widget::settings::item::builder(fl!("server-name"))
                .description(fl!("server-name-description"))
                .flex_control(
                    widget::text_input(fl!("server-name-placeholder"), &app.server_name_input)
                        .on_input(Message::ServerNameChanged)
                        .width(Length::Fixed(300.0)),
                ),
        )
        .add(
            cosmic::widget::settings::item::builder(fl!("server-url"))
                .description(fl!("server-url-description"))
                .flex_control(
                    widget::text_input(fl!("server-url-placeholder"), &app.server_url_input)
                        .on_input(Message::ServerUrlChanged)
                        .width(Length::Fixed(300.0)),
                ),
        )
        .add(
            cosmic::widget::settings::item::builder(fl!("auth-token"))
                .description(fl!("auth-token-description"))
                .flex_control(
                    widget::secure_input(
                        fl!("auth-token-placeholder"),
                        &app.auth_token_input,
                        None,
                        true,
                    )
                    .on_input(Message::AuthTokenChanged)
                    .width(Length::Fixed(300.0)),
                ),
        )
        .add(
            cosmic::widget::settings::item::builder(fl!("auth-header-type"))
                .description(fl!("auth-header-type-description"))
                .flex_control(
                    widget::row::with_capacity(2)
                        .push(
                            widget::button::text("Authorization: Bearer")
                                .class(if app.auth_header_type_input == "authorization" {
                                    cosmic::theme::Button::Suggested
                                } else {
                                    cosmic::theme::Button::Standard
                                })
                                .on_press(Message::AuthHeaderTypeChanged(
                                    "authorization".to_string(),
                                )),
                        )
                        .push(
                            widget::button::text("X-Chroma-Token")
                                .class(if app.auth_header_type_input == "x-chroma-token" {
                                    cosmic::theme::Button::Suggested
                                } else {
                                    cosmic::theme::Button::Standard
                                })
                                .on_press(Message::AuthHeaderTypeChanged(
                                    "x-chroma-token".to_string(),
                                )),
                        )
                        .spacing(space_s),
                ),
        )
        .add(
            cosmic::widget::settings::item::builder(fl!("tenant"))
                .description(fl!("tenant-description"))
                .flex_control({
                    let mut col = widget::column::with_capacity(3)
                        .push(
                            widget::row::with_capacity(2)
                                .push(
                                    widget::text_input(
                                        fl!("tenant-placeholder"),
                                        &app.tenant_input,
                                    )
                                    .on_input(Message::TenantChanged)
                                    .width(Length::Fixed(250.0)),
                                )
                                .push(
                                    widget::button::standard(fl!("load-tenants"))
                                        .on_press(Message::FetchTenants),
                                )
                                .spacing(space_s)
                                .align_y(Alignment::Center),
                        )
                        .spacing(space_s);

                    // Show dropdown if tenants are available
                    if !app.available_tenants.is_empty() {
                        let selected_idx = app
                            .available_tenants
                            .iter()
                            .position(|t| *t == app.tenant_input);
                        let tenants_clone = tenants_for_dropdown.clone();
                        col = col.push(
                            widget::dropdown(&app.available_tenants, selected_idx, move |idx| {
                                Message::SelectTenant(tenants_clone[idx].clone())
                            })
                            .width(Length::Fixed(300.0)),
                        );
                    }

                    // Show error if loading failed
                    if let Some(ref error) = app.tenants_load_error {
                        col = col.push(
                            widget::text::caption(format!("{}: {}", fl!("error"), error))
                                .class(cosmic::style::Text::Accent),
                        );
                    }

                    col
                }),
        )
        .add(
            cosmic::widget::settings::item::builder(fl!("database"))
                .description(fl!("database-description"))
                .flex_control({
                    let mut col = widget::column::with_capacity(3)
                        .push(
                            widget::row::with_capacity(2)
                                .push(
                                    widget::text_input(
                                        fl!("database-placeholder"),
                                        &app.database_input,
                                    )
                                    .on_input(Message::DatabaseChanged)
                                    .width(Length::Fixed(250.0)),
                                )
                                .push(
                                    widget::button::standard(fl!("load-databases"))
                                        .on_press(Message::FetchDatabases),
                                )
                                .spacing(space_s)
                                .align_y(Alignment::Center),
                        )
                        .spacing(space_s);

                    // Show dropdown if databases are available
                    if !app.available_databases.is_empty() {
                        let selected_idx = app
                            .available_databases
                            .iter()
                            .position(|d| *d == app.database_input);
                        let databases_clone = databases_for_dropdown.clone();
                        col = col.push(
                            widget::dropdown(&app.available_databases, selected_idx, move |idx| {
                                Message::SelectDatabase(databases_clone[idx].clone())
                            })
                            .width(Length::Fixed(300.0)),
                        );
                    }

                    // Show error if loading failed
                    if let Some(ref error) = app.databases_load_error {
                        col = col.push(
                            widget::text::caption(format!("{}: {}", fl!("error"), error))
                                .class(cosmic::style::Text::Accent),
                        );
                    }

                    col
                }),
        );

    // Add delete button if there's more than one server
    if app.config.servers.len() > 1 {
        server_section = server_section.add(
            cosmic::widget::settings::item::builder(fl!("delete-server"))
                .description(fl!("delete-server-description"))
                .flex_control(
                    widget::button::destructive(fl!("delete"))
                        .on_press(Message::DeleteServer(app.config.active_server)),
                ),
        );
    }

    // Settings save status - only need button labels and create button logic
    let (save_button_label, show_create_button) = match &app.settings_status {
        SettingsStatus::Idle | SettingsStatus::Saved | SettingsStatus::Error(_) => {
            (fl!("save"), false)
        }
        SettingsStatus::Validating => (fl!("validating"), false),
        SettingsStatus::MissingResources(_) => (fl!("save"), true),
        SettingsStatus::Creating => (fl!("creating"), false),
    };

    let save_button = if matches!(
        app.settings_status,
        SettingsStatus::Validating | SettingsStatus::Creating
    ) {
        widget::button::standard(save_button_label)
    } else {
        widget::button::standard(save_button_label).on_press(Message::ValidateAndSaveSettings)
    };

    let mut buttons = widget::row::with_capacity(3)
        .push(save_button)
        .push(widget::button::suggested(fl!("test-connection")).on_press(Message::TestConnection))
        .spacing(space_s)
        .align_y(Alignment::Center);

    // Show create button if resources are missing
    if show_create_button {
        buttons = buttons.push(
            widget::button::suggested(fl!("create-missing"))
                .on_press(Message::CreateMissingResources),
        );
    }

    widget::scrollable(
        widget::column::with_capacity(4)
            .push(header)
            .push(servers_section)
            .push(server_section)
            .push(buttons)
            .spacing(space_m)
            .width(Length::Fill),
    )
    .height(Length::Fill)
    .into()
}
