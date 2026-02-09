// SPDX-License-Identifier: MPL-2.0

use crate::api::{ChromaClient, Collection, Document, ServerInfo};
use crate::config::{Config, ServerConfig};
use crate::fl;
use cosmic::app::context_drawer;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::{Alignment, Length, Subscription};
use cosmic::widget::{self, about::About, icon, menu, nav_bar};
use cosmic::prelude::*;
use std::collections::HashMap;

const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const APP_ICON: &[u8] = include_bytes!("../resources/icons/hicolor/scalable/apps/icon.svg");

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
pub struct AppModel {
    /// Application state which is managed by the COSMIC runtime.
    core: cosmic::Core,
    /// Display a context drawer with the designated page if defined.
    context_page: ContextPage,
    /// The about page for this app.
    about: About,
    /// Contains items assigned to the nav bar panel.
    nav: nav_bar::Model,
    /// Key bindings for the application's menu bar.
    key_binds: HashMap<menu::KeyBind, MenuAction>,
    /// Configuration data that persists between application runs.
    config: Config,
    /// Cosmic config context for saving
    config_context: Option<cosmic_config::Config>,
    
    // === App-specific state ===
    /// List of collections from the server
    collections: Vec<Collection>,
    /// Connection status
    connection_status: ConnectionStatus,
    /// Temporary server name input (before saving)
    server_name_input: String,
    /// Temporary server URL input (before saving)
    server_url_input: String,
    /// Temporary auth token input (before saving)
    auth_token_input: String,
    /// Temporary auth header type input (before saving)
    auth_header_type_input: String,
    /// Temporary tenant input (before saving)
    tenant_input: String,
    /// Temporary database input (before saving)
    database_input: String,
    /// Index of server being edited (None for new server dialog)
    editing_server_index: Option<usize>,
    /// Currently selected collection
    selected_collection: Option<Collection>,
    /// Documents in the selected collection
    documents: Vec<Document>,
    /// Settings save/validation status
    settings_status: SettingsStatus,
    /// Server info for dashboard
    server_info: Option<ServerInfo>,
    /// Available databases for the current tenant (for selection)
    available_databases: Vec<String>,
}

/// What's missing during validation
#[derive(Debug, Clone)]
pub struct ValidationMissing {
    pub tenant_exists: bool,
    pub database_exists: bool,
    pub tenant_name: String,
    pub database_name: String,
}

#[derive(Debug, Clone, Default)]
pub enum SettingsStatus {
    #[default]
    Idle,
    Validating,
    Saved,
    Error(String),
    /// Tenant and/or database don't exist - offer to create them
    MissingResources(ValidationMissing),
    /// Creating tenant/database in progress
    Creating,
}

#[derive(Debug, Clone, Default)]
pub enum ConnectionStatus {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    // Navigation & UI
    LaunchUrl(String),
    ToggleContextPage(ContextPage),
    UpdateConfig(Config),
    
    // Settings inputs
    ServerNameChanged(String),
    ServerUrlChanged(String),
    AuthTokenChanged(String),
    AuthHeaderTypeChanged(String),
    TenantChanged(String),
    DatabaseChanged(String),
    SaveSettings,
    ValidateAndSaveSettings,
    /// Result contains (tenant_exists, database_exists) for detailed feedback
    SettingsValidationResult(Result<(), (bool, bool)>),
    /// Create missing tenant and/or database
    CreateMissingResources,
    CreateResourcesResult(Result<(), String>),
    /// Fetch available databases for current tenant
    FetchDatabases,
    DatabasesLoaded(Result<Vec<String>, String>),
    
    // Server management
    SelectServer(usize),
    AddNewServer,
    DeleteServer(usize),
    
    // Connection & data
    TestConnection,
    ConnectionResult(Result<(), String>),
    FetchCollections,
    CollectionsLoaded(Result<Vec<Collection>, String>),
    
    // Collection & documents
    SelectCollection(Collection),
    BackToCollections,
    FetchDocuments,
    DocumentsLoaded(Result<Vec<Document>, String>),
    
    // Dashboard
    FetchServerInfo,
    ServerInfoLoaded(Result<ServerInfo, String>),
}

/// Create a COSMIC application from the app model
impl cosmic::Application for AppModel {
    /// The async executor that will be used to run your application's commands.
    type Executor = cosmic::executor::Default;

    /// Data that your application receives to its init method.
    type Flags = ();

    /// Messages which the application and its widgets will emit.
    type Message = Message;

    /// Unique identifier in RDNN (reverse domain name notation) format.
    const APP_ID: &'static str = "dev.mmurphy.Chromatic";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    /// Initializes the application with any given flags and startup commands.
    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        // Create a nav bar with three pages: Dashboard, Collections and Settings
        let mut nav = nav_bar::Model::default();

        nav.insert()
            .text(fl!("dashboard"))
            .data::<Page>(Page::Dashboard)
            .icon(icon::from_name("utilities-system-monitor-symbolic"))
            .activate();

        nav.insert()
            .text(fl!("collections"))
            .data::<Page>(Page::Collections)
            .icon(icon::from_name("folder-symbolic"));

        nav.insert()
            .text(fl!("settings"))
            .data::<Page>(Page::Settings)
            .icon(icon::from_name("preferences-system-symbolic"));

        // Create the about widget
        let about = About::default()
            .name(fl!("app-title"))
            .icon(widget::icon::from_svg_bytes(APP_ICON))
            .version(env!("CARGO_PKG_VERSION"))
            .links([(fl!("repository"), REPOSITORY)])
            .license(env!("CARGO_PKG_LICENSE"));

        // Load configuration
        let config_context = cosmic_config::Config::new(Self::APP_ID, Config::VERSION).ok();
        let config = config_context
            .as_ref()
            .map(|context| match Config::get_entry(context) {
                Ok(config) => config,
                Err((_errors, config)) => config,
            })
            .unwrap_or_default();

        // Get active server config for initializing input fields
        let active = config.active_config();
        
        // Construct the app model with the runtime's core.
        let mut app = AppModel {
            core,
            context_page: ContextPage::default(),
            about,
            nav,
            key_binds: HashMap::new(),
            server_name_input: active.name.clone(),
            server_url_input: active.server_url.clone(),
            auth_token_input: active.auth_token.clone(),
            auth_header_type_input: active.auth_header_type.clone(),
            tenant_input: active.tenant.clone(),
            database_input: active.database.clone(),
            editing_server_index: Some(config.active_server),
            config,
            config_context,
            collections: Vec::new(),
            connection_status: ConnectionStatus::Disconnected,
            selected_collection: None,
            documents: Vec::new(),
            settings_status: SettingsStatus::Idle,
            server_info: None,
            available_databases: Vec::new(),
        };

        // Create a startup command that sets the window title.
        let command = app.update_title();

        (app, command)
    }

    /// Elements to pack at the start of the header bar.
    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        let menu_bar = menu::bar(vec![menu::Tree::with_children(
            menu::root(fl!("view")).apply(Element::from),
            menu::items(
                &self.key_binds,
                vec![menu::Item::Button(fl!("about"), None, MenuAction::About)],
            ),
        )]);

        vec![menu_bar.into()]
    }

    /// Enables the COSMIC application to create a nav bar with this model.
    fn nav_model(&self) -> Option<&nav_bar::Model> {
        Some(&self.nav)
    }

    /// Display a context drawer if the context page is requested.
    fn context_drawer(&self) -> Option<context_drawer::ContextDrawer<'_, Self::Message>> {
        if !self.core.window.show_context {
            return None;
        }

        Some(match self.context_page {
            ContextPage::About => context_drawer::about(
                &self.about,
                |url| Message::LaunchUrl(url.to_string()),
                Message::ToggleContextPage(ContextPage::About),
            ),
        })
    }

    /// Describes the interface based on the current state of the application model.
    fn view(&self) -> Element<'_, Self::Message> {
        let space_s = cosmic::theme::spacing().space_s;
        let space_m = cosmic::theme::spacing().space_m;
        
        let content: Element<_> = match self.nav.active_data::<Page>().unwrap_or(&Page::Dashboard) {
            Page::Dashboard => self.view_dashboard(space_s, space_m),
            Page::Collections => {
                // Show documents view if a collection is selected
                if self.selected_collection.is_some() {
                    self.view_documents(space_s, space_m)
                } else {
                    self.view_collections(space_s, space_m)
                }
            }
            Page::Settings => self.view_settings(space_s, space_m),
        };

        widget::container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(space_m)
            .into()
    }

    /// Register subscriptions for this application.
    fn subscription(&self) -> Subscription<Self::Message> {
        // Watch for application configuration changes.
        self.core()
            .watch_config::<Config>(Self::APP_ID)
            .map(|update| Message::UpdateConfig(update.config))
    }

    /// Handles messages emitted by the application and its widgets.
    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::ToggleContextPage(context_page) => {
                if self.context_page == context_page {
                    self.core.window.show_context = !self.core.window.show_context;
                } else {
                    self.context_page = context_page;
                    self.core.window.show_context = true;
                }
            }

            Message::UpdateConfig(config) => {
                self.config = config;
                let active = self.config.active_config();
                self.server_name_input = active.name.clone();
                self.server_url_input = active.server_url.clone();
                self.auth_token_input = active.auth_token.clone();
                self.auth_header_type_input = active.auth_header_type.clone();
                self.tenant_input = active.tenant.clone();
                self.database_input = active.database.clone();
                self.editing_server_index = Some(self.config.active_server);
            }

            Message::LaunchUrl(url) => match open::that_detached(&url) {
                Ok(()) => {}
                Err(err) => {
                    eprintln!("failed to open {url:?}: {err}");
                }
            },

            // Settings inputs
            Message::ServerUrlChanged(url) => {
                self.server_url_input = url;
            }

            Message::AuthTokenChanged(token) => {
                self.auth_token_input = token;
            }

            Message::AuthHeaderTypeChanged(header_type) => {
                self.auth_header_type_input = header_type;
            }

            Message::TenantChanged(tenant) => {
                self.tenant_input = tenant;
            }

            Message::DatabaseChanged(database) => {
                self.database_input = database;
            }

            Message::ServerNameChanged(name) => {
                self.server_name_input = name;
            }

            Message::SelectServer(index) => {
                if self.config.switch_active_server(index) {
                    // Save the config with new active server
                    if let Some(ref context) = self.config_context {
                        let _ = self.config.write_entry(context);
                    }
                    // Update input fields with the new server's config
                    let active = self.config.active_config();
                    self.server_name_input = active.name.clone();
                    self.server_url_input = active.server_url.clone();
                    self.auth_token_input = active.auth_token.clone();
                    self.auth_header_type_input = active.auth_header_type.clone();
                    self.tenant_input = active.tenant.clone();
                    self.database_input = active.database.clone();
                    self.editing_server_index = Some(index);
                    // Clear cached data from previous server
                    self.collections.clear();
                    self.server_info = None;
                    self.connection_status = ConnectionStatus::Disconnected;
                }
            }

            Message::AddNewServer => {
                // Create a new server with default values and a unique name
                let new_name = format!("Server {}", self.config.servers.len() + 1);
                let new_server = ServerConfig::new(&new_name);
                let new_index = self.config.add_server(new_server);
                // Switch to the new server
                self.config.switch_active_server(new_index);
                // Save the config
                if let Some(ref context) = self.config_context {
                    let _ = self.config.write_entry(context);
                }
                // Update input fields
                let active = self.config.active_config();
                self.server_name_input = active.name.clone();
                self.server_url_input = active.server_url.clone();
                self.auth_token_input = active.auth_token.clone();
                self.auth_header_type_input = active.auth_header_type.clone();
                self.tenant_input = active.tenant.clone();
                self.database_input = active.database.clone();
                self.editing_server_index = Some(new_index);
                // Clear cached data
                self.collections.clear();
                self.server_info = None;
                self.connection_status = ConnectionStatus::Disconnected;
            }

            Message::DeleteServer(index) => {
                if self.config.remove_server(index) {
                    // Save the config
                    if let Some(ref context) = self.config_context {
                        let _ = self.config.write_entry(context);
                    }
                    // Update input fields with the (possibly new) active server
                    let active = self.config.active_config();
                    self.server_name_input = active.name.clone();
                    self.server_url_input = active.server_url.clone();
                    self.auth_token_input = active.auth_token.clone();
                    self.auth_header_type_input = active.auth_header_type.clone();
                    self.tenant_input = active.tenant.clone();
                    self.database_input = active.database.clone();
                    self.editing_server_index = Some(self.config.active_server);
                    // Clear cached data
                    self.collections.clear();
                    self.server_info = None;
                    self.connection_status = ConnectionStatus::Disconnected;
                }
            }

            Message::SaveSettings => {
                // Direct save without validation (internal use)
                {
                    let active = self.config.active_config_mut();
                    active.name = self.server_name_input.clone();
                    active.server_url = self.server_url_input.clone();
                    active.auth_token = self.auth_token_input.clone();
                    active.auth_header_type = self.auth_header_type_input.clone();
                    active.tenant = self.tenant_input.clone();
                    active.database = self.database_input.clone();
                }
                
                if let Some(ref context) = self.config_context {
                    if let Err(e) = self.config.write_entry(context) {
                        eprintln!("Failed to save config: {}", e);
                        self.settings_status = SettingsStatus::Error(format!("Failed to save: {}", e));
                    } else {
                        self.settings_status = SettingsStatus::Saved;
                    }
                }
            }

            Message::ValidateAndSaveSettings => {
                self.settings_status = SettingsStatus::Validating;
                let url = self.server_url_input.clone();
                let token = self.auth_token_input.clone();
                let auth_header_type = self.auth_header_type_input.clone();
                let tenant = self.tenant_input.clone();
                let database = self.database_input.clone();
                
                return cosmic::task::future(async move {
                    let result = validate_tenant_database(&url, &token, &auth_header_type, &tenant, &database).await;
                    cosmic::Action::App(Message::SettingsValidationResult(result))
                });
            }

            Message::SettingsValidationResult(result) => {
                match result {
                    Ok(()) => {
                        // Validation passed, save the settings
                        return self.update(Message::SaveSettings);
                    }
                    Err((tenant_exists, database_exists)) => {
                        // Show what's missing and offer to create
                        self.settings_status = SettingsStatus::MissingResources(ValidationMissing {
                            tenant_exists,
                            database_exists,
                            tenant_name: self.tenant_input.clone(),
                            database_name: self.database_input.clone(),
                        });
                    }
                }
            }

            Message::CreateMissingResources => {
                // Extract the missing info before reassigning settings_status
                let missing_info = if let SettingsStatus::MissingResources(missing) = &self.settings_status {
                    Some((missing.tenant_exists, missing.database_exists))
                } else {
                    None
                };
                
                if let Some((tenant_exists, database_exists)) = missing_info {
                    self.settings_status = SettingsStatus::Creating;
                    let url = self.server_url_input.clone();
                    let token = self.auth_token_input.clone();
                    let auth_header_type = self.auth_header_type_input.clone();
                    let tenant = self.tenant_input.clone();
                    let database = self.database_input.clone();
                    
                    return cosmic::task::future(async move {
                        let result = create_missing_resources(&url, &token, &auth_header_type, &tenant, &database, tenant_exists, database_exists).await;
                        cosmic::Action::App(Message::CreateResourcesResult(result))
                    });
                }
            }

            Message::CreateResourcesResult(result) => {
                match result {
                    Ok(()) => {
                        // Resources created successfully, now save settings
                        return self.update(Message::SaveSettings);
                    }
                    Err(e) => {
                        self.settings_status = SettingsStatus::Error(format!("Failed to create resources: {}", e));
                    }
                }
            }

            Message::FetchDatabases => {
                let url = self.server_url_input.clone();
                let token = self.auth_token_input.clone();
                let auth_header_type = self.auth_header_type_input.clone();
                let tenant = self.tenant_input.clone();
                
                return cosmic::task::future(async move {
                    let result = fetch_databases(&url, &token, &auth_header_type, &tenant).await;
                    cosmic::Action::App(Message::DatabasesLoaded(result))
                });
            }

            Message::DatabasesLoaded(result) => {
                match result {
                    Ok(databases) => {
                        self.available_databases = databases;
                    }
                    Err(_) => {
                        // Silently fail - databases list is optional
                        self.available_databases.clear();
                    }
                }
            }

            Message::TestConnection => {
                self.connection_status = ConnectionStatus::Connecting;
                let url = self.server_url_input.clone();
                let token = self.auth_token_input.clone();
                let auth_header_type = self.auth_header_type_input.clone();
                
                return cosmic::task::future(async move {
                    let result = test_connection(&url, &token, &auth_header_type).await;
                    cosmic::Action::App(Message::ConnectionResult(result))
                });
            }

            Message::ConnectionResult(result) => {
                match result {
                    Ok(()) => {
                        self.connection_status = ConnectionStatus::Connected;
                    }
                    Err(e) => {
                        self.connection_status = ConnectionStatus::Error(e);
                    }
                }
            }

            Message::FetchCollections => {
                self.connection_status = ConnectionStatus::Connecting;
                let active = self.config.active_config();
                let url = active.server_url.clone();
                let token = active.auth_token.clone();
                let auth_header_type = active.auth_header_type.clone();
                let tenant = active.tenant.clone();
                let database = active.database.clone();
                
                return cosmic::task::future(async move {
                    let result = fetch_collections(&url, &token, &auth_header_type, &tenant, &database).await;
                    cosmic::Action::App(Message::CollectionsLoaded(result))
                });
            }

            Message::CollectionsLoaded(result) => {
                match result {
                    Ok(collections) => {
                        self.collections = collections;
                        self.connection_status = ConnectionStatus::Connected;
                    }
                    Err(e) => {
                        self.connection_status = ConnectionStatus::Error(e);
                    }
                }
            }

            Message::SelectCollection(collection) => {
                self.selected_collection = Some(collection);
                self.documents.clear();
                // Automatically fetch documents when selecting a collection
                return self.update(Message::FetchDocuments);
            }

            Message::BackToCollections => {
                self.selected_collection = None;
                self.documents.clear();
            }

            Message::FetchDocuments => {
                if let Some(ref collection) = self.selected_collection {
                    self.connection_status = ConnectionStatus::Connecting;
                    let active = self.config.active_config();
                    let url = active.server_url.clone();
                    let token = active.auth_token.clone();
                    let auth_header_type = active.auth_header_type.clone();
                    let collection_id = collection.id.clone();
                    let tenant = active.tenant.clone();
                    let database = active.database.clone();
                    
                    return cosmic::task::future(async move {
                        let result = fetch_documents(&url, &token, &auth_header_type, &collection_id, &tenant, &database).await;
                        cosmic::Action::App(Message::DocumentsLoaded(result))
                    });
                }
            }

            Message::DocumentsLoaded(result) => {
                match result {
                    Ok(documents) => {
                        self.documents = documents;
                        self.connection_status = ConnectionStatus::Connected;
                    }
                    Err(e) => {
                        self.connection_status = ConnectionStatus::Error(e);
                    }
                }
            }

            Message::FetchServerInfo => {
                self.connection_status = ConnectionStatus::Connecting;
                let active = self.config.active_config();
                let url = active.server_url.clone();
                let token = active.auth_token.clone();
                let auth_header_type = active.auth_header_type.clone();
                
                return cosmic::task::future(async move {
                    let result = fetch_server_info(&url, &token, &auth_header_type).await;
                    cosmic::Action::App(Message::ServerInfoLoaded(result))
                });
            }

            Message::ServerInfoLoaded(result) => {
                match result {
                    Ok(info) => {
                        self.server_info = Some(info);
                        self.connection_status = ConnectionStatus::Connected;
                        // Also fetch collections count for the dashboard
                        return self.update(Message::FetchCollections);
                    }
                    Err(e) => {
                        self.server_info = None;
                        self.connection_status = ConnectionStatus::Error(e);
                    }
                }
            }
        }
        Task::none()
    }

    /// Called when a nav item is selected.
    fn on_nav_select(&mut self, id: nav_bar::Id) -> Task<cosmic::Action<Self::Message>> {
        self.nav.activate(id);
        self.update_title()
    }
}

impl AppModel {
    /// Updates the header and window titles.
    pub fn update_title(&mut self) -> Task<cosmic::Action<Message>> {
        let mut window_title = fl!("app-title");

        if let Some(page) = self.nav.text(self.nav.active()) {
            window_title.push_str(" — ");
            window_title.push_str(page);
        }

        if let Some(id) = self.core.main_window_id() {
            self.set_window_title(window_title, id)
        } else {
            Task::none()
        }
    }

    /// View for the Dashboard page
    fn view_dashboard(&self, _space_s: u16, space_m: u16) -> Element<'_, Message> {
        let header = widget::row::with_capacity(2)
            .push(widget::text::title1(fl!("dashboard")))
            .push(self.connection_status_badge())
            .align_y(Alignment::Center)
            .spacing(space_m);

        let refresh_button = widget::button::standard(fl!("refresh"))
            .on_press(Message::FetchServerInfo);

        // Stats cards
        let version_card = self.stat_card(
            fl!("server-version"),
            self.server_info.as_ref().map(|i| i.version.clone()).unwrap_or_else(|| "-".to_string()),
        );

        let heartbeat_card = self.stat_card(
            fl!("heartbeat"),
            self.server_info.as_ref().map(|i| {
                // Convert nanoseconds to a readable format
                let secs = i.heartbeat_ns / 1_000_000_000;
                format!("{} s", secs)
            }).unwrap_or_else(|| "-".to_string()),
        );

        let active = self.config.active_config();
        let tenant_card = self.stat_card(
            fl!("current-tenant"),
            active.tenant.clone(),
        );

        let database_card = self.stat_card(
            fl!("current-database"),
            active.database.clone(),
        );

        let collections_card = self.stat_card(
            fl!("collection-count"),
            self.collections.len().to_string(),
        );

        let api_version_card = self.stat_card(
            fl!("api-version"),
            self.server_info.as_ref().map(|i| i.api_version.clone()).unwrap_or_else(|| "-".to_string()),
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

        let content: Element<'_, Message> = match &self.connection_status {
            ConnectionStatus::Disconnected | ConnectionStatus::Error(_) => {
                widget::column::with_capacity(2)
                    .push(
                        widget::container(
                            widget::text::body(fl!("dashboard-connect-hint"))
                        )
                        .padding(space_m)
                        .width(Length::Fill)
                        .class(cosmic::style::Container::Card)
                    )
                    .push(refresh_button)
                    .spacing(space_m)
                    .into()
            }
            _ => {
                widget::column::with_capacity(3)
                    .push(refresh_button)
                    .push(stats_row1)
                    .push(stats_row2)
                    .spacing(space_m)
                    .into()
            }
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
    fn stat_card(&self, label: String, value: String) -> Element<'_, Message> {
        widget::container(
            widget::column::with_capacity(2)
                .push(widget::text::caption(label))
                .push(widget::text::title3(value))
                .spacing(4)
        )
        .padding(cosmic::theme::spacing().space_s)
        .width(Length::FillPortion(1))
        .class(cosmic::style::Container::Card)
        .into()
    }

    /// View for the Collections page
    fn view_collections(&self, space_s: u16, space_m: u16) -> Element<'_, Message> {
        let header = widget::row::with_capacity(2)
            .push(widget::text::title1(fl!("collections")))
            .push(self.connection_status_badge())
            .align_y(Alignment::Center)
            .spacing(space_m);

        let refresh_button = widget::button::standard(fl!("refresh"))
            .on_press(Message::FetchCollections);

        let toolbar = widget::row::with_capacity(1)
            .push(refresh_button)
            .spacing(space_s);

        let content: Element<'_, Message> = if self.collections.is_empty() {
            let empty_message = match &self.connection_status {
                ConnectionStatus::Disconnected => fl!("not-connected"),
                ConnectionStatus::Connecting => fl!("connecting"),
                ConnectionStatus::Connected => fl!("no-collections"),
                ConnectionStatus::Error(e) => e.clone(),
            };
            
            widget::container(
                widget::text::body(empty_message)
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .into()
        } else {
            let mut list_column = widget::column::with_capacity(self.collections.len());
            
            for collection in &self.collections {
                let collection_clone = collection.clone();
                let item = widget::mouse_area(
                    widget::container(
                        widget::column::with_capacity(2)
                            .push(widget::text::title4(&collection.name))
                            .push(widget::text::caption(format!("ID: {}", collection.id)))
                            .spacing(4)
                    )
                    .padding(space_s)
                    .width(Length::Fill)
                    .class(cosmic::style::Container::Card)
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

    /// View for the Documents page (when a collection is selected)
    fn view_documents(&self, space_s: u16, space_m: u16) -> Element<'_, Message> {
        let collection_name = self
            .selected_collection
            .as_ref()
            .map(|c| c.name.as_str())
            .unwrap_or("Unknown");

        let back_button = widget::button::icon(icon::from_name("go-previous-symbolic"))
            .on_press(Message::BackToCollections);

        let header = widget::row::with_capacity(3)
            .push(back_button)
            .push(widget::text::title1(collection_name))
            .push(self.connection_status_badge())
            .align_y(Alignment::Center)
            .spacing(space_m);

        let refresh_button = widget::button::standard(fl!("refresh"))
            .on_press(Message::FetchDocuments);

        let doc_count = widget::text::body(format!("{} {}", self.documents.len(), fl!("documents-count")));

        let toolbar = widget::row::with_capacity(2)
            .push(refresh_button)
            .push(doc_count)
            .spacing(space_s)
            .align_y(Alignment::Center);

        let content: Element<'_, Message> = if self.documents.is_empty() {
            let empty_message = match &self.connection_status {
                ConnectionStatus::Disconnected => fl!("not-connected"),
                ConnectionStatus::Connecting => fl!("loading-documents"),
                ConnectionStatus::Connected => fl!("no-documents"),
                ConnectionStatus::Error(e) => e.clone(),
            };
            
            widget::container(
                widget::text::body(empty_message)
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(Horizontal::Center)
            .align_y(Vertical::Center)
            .into()
        } else {
            let mut list_column = widget::column::with_capacity(self.documents.len());
            
            for doc in &self.documents {
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
                    item_content = item_content.push(
                        widget::text::caption(metadata_str)
                    );
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

    /// View for the Settings page
    fn view_settings(&self, space_s: u16, space_m: u16) -> Element<'_, Message> {
        let header = widget::text::title1(fl!("settings"));

        // Server selection section - show list of saved servers
        let mut server_buttons = widget::row::with_capacity(self.config.servers.len() + 1);
        for (index, server) in self.config.servers.iter().enumerate() {
            let is_active = index == self.config.active_server;
            let button = widget::button::text(&server.name)
                .class(if is_active {
                    cosmic::theme::Button::Suggested
                } else {
                    cosmic::theme::Button::Standard
                })
                .on_press(Message::SelectServer(index));
            server_buttons = server_buttons.push(button);
        }
        // Add new server button
        server_buttons = server_buttons
            .push(widget::button::icon(icon::from_name("list-add-symbolic")).on_press(Message::AddNewServer))
            .spacing(space_s);

        let servers_section = cosmic::widget::settings::section()
            .title(fl!("servers"))
            .add(
                cosmic::widget::settings::item::builder(fl!("saved-servers"))
                    .description(fl!("saved-servers-description"))
                    .control(server_buttons)
            );

        // Server configuration section
        let mut server_section = cosmic::widget::settings::section()
            .title(fl!("server-config"))
            .add(
                cosmic::widget::settings::item::builder(fl!("server-name"))
                    .description(fl!("server-name-description"))
                    .control(
                        widget::text_input(fl!("server-name-placeholder"), &self.server_name_input)
                            .on_input(Message::ServerNameChanged)
                            .width(Length::Fixed(300.0))
                    )
            )
            .add(
                cosmic::widget::settings::item::builder(fl!("server-url"))
                    .description(fl!("server-url-description"))
                    .control(
                        widget::text_input(fl!("server-url-placeholder"), &self.server_url_input)
                            .on_input(Message::ServerUrlChanged)
                            .width(Length::Fixed(300.0))
                    )
            )
            .add(
                cosmic::widget::settings::item::builder(fl!("auth-token"))
                    .description(fl!("auth-token-description"))
                    .control(
                        widget::secure_input(fl!("auth-token-placeholder"), &self.auth_token_input, None, true)
                            .on_input(Message::AuthTokenChanged)
                            .width(Length::Fixed(300.0))
                    )
            )
            .add(
                cosmic::widget::settings::item::builder(fl!("auth-header-type"))
                    .description(fl!("auth-header-type-description"))
                    .control(
                        widget::row::with_capacity(2)
                            .push(
                                widget::button::text("Authorization: Bearer")
                                    .class(if self.auth_header_type_input == "authorization" {
                                        cosmic::theme::Button::Suggested
                                    } else {
                                        cosmic::theme::Button::Standard
                                    })
                                    .on_press(Message::AuthHeaderTypeChanged("authorization".to_string()))
                            )
                            .push(
                                widget::button::text("X-Chroma-Token")
                                    .class(if self.auth_header_type_input == "x-chroma-token" {
                                        cosmic::theme::Button::Suggested
                                    } else {
                                        cosmic::theme::Button::Standard
                                    })
                                    .on_press(Message::AuthHeaderTypeChanged("x-chroma-token".to_string()))
                            )
                            .spacing(space_s)
                    )
            )
            .add(
                cosmic::widget::settings::item::builder(fl!("tenant"))
                    .description(fl!("tenant-description"))
                    .control(
                        widget::text_input(fl!("tenant-placeholder"), &self.tenant_input)
                            .on_input(Message::TenantChanged)
                            .width(Length::Fixed(300.0))
                    )
            )
            .add(
                cosmic::widget::settings::item::builder(fl!("database"))
                    .description(fl!("database-description"))
                    .control(
                        widget::text_input(fl!("database-placeholder"), &self.database_input)
                            .on_input(Message::DatabaseChanged)
                            .width(Length::Fixed(300.0))
                    )
            );

        // Add delete button if there's more than one server
        if self.config.servers.len() > 1 {
            server_section = server_section.add(
                cosmic::widget::settings::item::builder(fl!("delete-server"))
                    .description(fl!("delete-server-description"))
                    .control(
                        widget::button::destructive(fl!("delete"))
                            .on_press(Message::DeleteServer(self.config.active_server))
                    )
            );
        }

        // Connection status
        let connection_status_text = match &self.connection_status {
            ConnectionStatus::Disconnected => fl!("status-disconnected"),
            ConnectionStatus::Connecting => fl!("status-connecting"),
            ConnectionStatus::Connected => fl!("status-connected"),
            ConnectionStatus::Error(e) => format!("{}: {}", fl!("status-error"), e),
        };

        // Settings save status
        let (save_button_label, save_status_text, show_create_button) = match &self.settings_status {
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

        let save_button = if matches!(self.settings_status, SettingsStatus::Validating | SettingsStatus::Creating) {
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
                    .on_press(Message::CreateMissingResources)
            );
        }

        // Show save status if there's one
        if !save_status_text.is_empty() {
            let status_style = match &self.settings_status {
                SettingsStatus::Saved => cosmic::theme::Button::Suggested,
                SettingsStatus::Error(_) | SettingsStatus::MissingResources(_) => cosmic::theme::Button::Destructive,
                _ => cosmic::theme::Button::Standard,
            };
            buttons = buttons.push(
                widget::button::custom(widget::text::caption(save_status_text))
                    .class(status_style)
            );
        }

        widget::scrollable(
            widget::column::with_capacity(4)
                .push(header)
                .push(servers_section)
                .push(server_section)
                .push(buttons)
                .spacing(space_m)
                .width(Length::Fill)
        )
        .height(Length::Fill)
        .into()
    }

    /// Connection status badge widget
    fn connection_status_badge(&self) -> Element<'_, Message> {
        let (text, style) = match &self.connection_status {
            ConnectionStatus::Disconnected => (fl!("disconnected"), cosmic::theme::Button::Standard),
            ConnectionStatus::Connecting => (fl!("connecting"), cosmic::theme::Button::Standard),
            ConnectionStatus::Connected => (fl!("connected"), cosmic::theme::Button::Suggested),
            ConnectionStatus::Error(_) => (fl!("error"), cosmic::theme::Button::Destructive),
        };
        
        widget::button::custom(widget::text::body(text))
            .class(style)
            .into()
    }
}

/// The page to display in the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    Dashboard,
    Collections,
    Settings,
}

/// The context page to display in the context drawer.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum ContextPage {
    #[default]
    About,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuAction {
    About,
}

impl menu::action::MenuAction for MenuAction {
    type Message = Message;

    fn message(&self) -> Self::Message {
        match self {
            MenuAction::About => Message::ToggleContextPage(ContextPage::About),
        }
    }
}

// === Async helper functions ===

/// Helper to create a client with auto-detected API version
async fn create_client(url: &str, token: &str, auth_header_type: &str) -> Result<ChromaClient, String> {
    let api_version = ChromaClient::detect_api_version(url, token, auth_header_type)
        .await
        .map_err(|e| e.to_string())?;
    ChromaClient::new(url, token, auth_header_type, api_version).map_err(|e| e.to_string())
}

async fn test_connection(url: &str, token: &str, auth_header_type: &str) -> Result<(), String> {
    // Just detect API version - if it succeeds, connection works
    let _api_version = ChromaClient::detect_api_version(url, token, auth_header_type)
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

async fn fetch_server_info(url: &str, token: &str, auth_header_type: &str) -> Result<ServerInfo, String> {
    let client = create_client(url, token, auth_header_type).await?;
    client.get_server_info().await.map_err(|e| e.to_string())
}

/// Validate tenant and database, returning (tenant_exists, database_exists) on failure
async fn validate_tenant_database(url: &str, token: &str, auth_header_type: &str, tenant: &str, database: &str) -> Result<(), (bool, bool)> {
    let client = create_client(url, token, auth_header_type).await.map_err(|_| (false, false))?;
    let (tenant_exists, database_exists) = client.check_tenant_database_status(tenant, database).await;
    if tenant_exists && database_exists {
        Ok(())
    } else {
        Err((tenant_exists, database_exists))
    }
}

/// Create missing tenant and/or database
async fn create_missing_resources(url: &str, token: &str, auth_header_type: &str, tenant: &str, database: &str, tenant_exists: bool, database_exists: bool) -> Result<(), String> {
    let client = create_client(url, token, auth_header_type).await?;
    
    // Create tenant if needed
    if !tenant_exists {
        client.create_tenant(tenant).await.map_err(|e| e.to_string())?;
    }
    
    // Create database if needed
    if !database_exists {
        client.create_database(tenant, database).await.map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

/// Fetch available databases for a tenant
async fn fetch_databases(url: &str, token: &str, auth_header_type: &str, tenant: &str) -> Result<Vec<String>, String> {
    let client = create_client(url, token, auth_header_type).await?;
    let databases = client.list_databases(tenant).await.map_err(|e| e.to_string())?;
    Ok(databases.into_iter().map(|db| db.name).collect())
}

async fn fetch_collections(url: &str, token: &str, auth_header_type: &str, tenant: &str, database: &str) -> Result<Vec<Collection>, String> {
    let client = create_client(url, token, auth_header_type).await?;
    client.list_collections(tenant, database).await.map_err(|e| e.to_string())
}

async fn fetch_documents(url: &str, token: &str, auth_header_type: &str, collection_id: &str, tenant: &str, database: &str) -> Result<Vec<Document>, String> {
    let client = create_client(url, token, auth_header_type).await?;
    client.get_documents(collection_id, Some(100), None, tenant, database).await.map_err(|e| e.to_string())
}
