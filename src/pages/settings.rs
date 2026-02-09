// SPDX-License-Identifier: MPL-2.0

//! Settings page view for the Chromatic application.

use crate::app::{AppModel, ConnectionStatus, Message, SettingsStatus};
use crate::fl;
use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::widget::{self, icon};

/// View for the Settings page
pub fn view(app: &AppModel, space_s: u16, space_m: u16) -> Element<'_, Message> {
    let header = widget::text::title1(fl!("settings"));

    // Server selection section - show dropdown-style selector of saved servers
    let mut server_selector = widget::row::with_capacity(app.config.servers.len() + 1);
    for (index, server) in app.config.servers.iter().enumerate() {
        let is_active = index == app.config.active_server;
        let button = widget::button::text(&server.name)
            .class(if is_active {
                cosmic::theme::Button::Suggested
            } else {
                cosmic::theme::Button::Standard
            })
            .on_press(Message::SelectServer(index));
        server_selector = server_selector.push(button);
    }
    server_selector = server_selector
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
                .control(server_selector),
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
                .control(
                    widget::text_input(fl!("server-name-placeholder"), &app.server_name_input)
                        .on_input(Message::ServerNameChanged)
                        .width(Length::Fixed(300.0)),
                ),
        )
        .add(
            cosmic::widget::settings::item::builder(fl!("server-url"))
                .description(fl!("server-url-description"))
                .control(
                    widget::text_input(fl!("server-url-placeholder"), &app.server_url_input)
                        .on_input(Message::ServerUrlChanged)
                        .width(Length::Fixed(300.0)),
                ),
        )
        .add(
            cosmic::widget::settings::item::builder(fl!("auth-token"))
                .description(fl!("auth-token-description"))
                .control(
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
                .control(
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
                .control({
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
                .control({
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
                .control(
                    widget::button::destructive(fl!("delete"))
                        .on_press(Message::DeleteServer(app.config.active_server)),
                ),
        );
    }

    // Connection status
    let connection_status_text = match &app.connection_status {
        ConnectionStatus::Disconnected => fl!("status-disconnected"),
        ConnectionStatus::Connecting => fl!("status-connecting"),
        ConnectionStatus::Connected => fl!("status-connected"),
        ConnectionStatus::Error(e) => format!("{}: {}", fl!("status-error"), e),
    };

    // Settings save status
    let (save_button_label, save_status_text, show_create_button) = match &app.settings_status {
        SettingsStatus::Idle => (fl!("save"), String::new(), false),
        SettingsStatus::Validating => (fl!("validating"), fl!("validating-tenant-db"), false),
        SettingsStatus::Saved => (fl!("save"), fl!("settings-saved"), false),
        SettingsStatus::Error(e) => (fl!("save"), e.clone(), false),
        SettingsStatus::MissingResources(missing) => {
            let mut missing_parts = Vec::new();
            if !missing.tenant_exists {
                missing_parts.push(format!("{} '{}'", fl!("tenant"), missing.tenant_name));
            }
            if !missing.database_exists {
                missing_parts.push(format!("{} '{}'", fl!("database"), missing.database_name));
            }
            let msg = format!("{}: {}", fl!("missing-resources"), missing_parts.join(", "));
            (fl!("save"), msg, true)
        }
        SettingsStatus::Creating => (fl!("creating"), fl!("creating-resources"), false),
    };

    let save_button = if matches!(
        app.settings_status,
        SettingsStatus::Validating | SettingsStatus::Creating
    ) {
        widget::button::standard(save_button_label)
    } else {
        widget::button::standard(save_button_label).on_press(Message::ValidateAndSaveSettings)
    };

    let mut buttons = widget::row::with_capacity(5)
        .push(save_button)
        .push(widget::button::suggested(fl!("test-connection")).on_press(Message::TestConnection))
        .push(widget::text::body(connection_status_text))
        .spacing(space_s)
        .align_y(Alignment::Center);

    // Show create button if resources are missing
    if show_create_button {
        buttons = buttons.push(
            widget::button::suggested(fl!("create-missing"))
                .on_press(Message::CreateMissingResources),
        );
    }

    // Show save status if there's one
    if !save_status_text.is_empty() {
        let status_style = match &app.settings_status {
            SettingsStatus::Saved => cosmic::theme::Button::Suggested,
            SettingsStatus::Error(_) | SettingsStatus::MissingResources(_) => {
                cosmic::theme::Button::Destructive
            }
            _ => cosmic::theme::Button::Standard,
        };
        buttons = buttons.push(
            widget::button::custom(widget::text::caption(save_status_text)).class(status_style),
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
