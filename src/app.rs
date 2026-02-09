// SPDX-License-Identifier: MPL-2.0

use crate::api::{Collection, Document, ServerInfo};
use crate::config::{Config, ServerConfig};
use crate::fl;
use crate::helpers;
use crate::pages;
use cosmic::app::context_drawer;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::{Length, Subscription};
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
    pub config: Config,
    /// Cosmic config context for saving
    config_context: Option<cosmic_config::Config>,
    
    // === App-specific state ===
    /// List of collections from the server
    pub collections: Vec<Collection>,
    /// Connection status
    pub connection_status: ConnectionStatus,
    /// Temporary server name input (before saving)
    pub server_name_input: String,
    /// Temporary server URL input (before saving)
    pub server_url_input: String,
    /// Temporary auth token input (before saving)
    pub auth_token_input: String,
    /// Temporary auth header type input (before saving)
    pub auth_header_type_input: String,
    /// Temporary tenant input (before saving)
    pub tenant_input: String,
    /// Temporary database input (before saving)
    pub database_input: String,
    /// Index of server being edited (None for new server dialog)
    pub editing_server_index: Option<usize>,
    /// Currently selected collection
    pub selected_collection: Option<Collection>,
    /// Documents in the selected collection
    pub documents: Vec<Document>,
    /// Settings save/validation status
    pub settings_status: SettingsStatus,
    /// Server info for dashboard
    pub server_info: Option<ServerInfo>,
    /// Available databases for the current tenant (for selection)
    pub available_databases: Vec<String>,
    /// Available tenants (for selection)
    pub available_tenants: Vec<String>,
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
    /// Fetch available tenants
    FetchTenants,
    TenantsLoaded(Result<Vec<String>, String>),
    /// Fetch available databases for current tenant
    FetchDatabases,
    DatabasesLoaded(Result<Vec<String>, String>),
    /// Select a tenant from the list
    SelectTenant(String),
    /// Select a database from the list
    SelectDatabase(String),
    
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
            available_tenants: Vec::new(),
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
            Page::Dashboard => pages::dashboard::view(self, space_s, space_m),
            Page::Collections => {
                // Show documents view if a collection is selected
                if self.selected_collection.is_some() {
                    pages::documents::view(self, space_s, space_m)
                } else {
                    pages::collections::view(self, space_s, space_m)
                }
            }
            Page::Settings => pages::settings::view(self, space_s, space_m),
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
                    let result = helpers::validate_tenant_database(&url, &token, &auth_header_type, &tenant, &database).await;
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
                        let result = helpers::create_missing_resources(&url, &token, &auth_header_type, &tenant, &database, tenant_exists, database_exists).await;
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
                    let result = helpers::fetch_databases(&url, &token, &auth_header_type, &tenant).await;
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

            Message::FetchTenants => {
                let url = self.server_url_input.clone();
                let token = self.auth_token_input.clone();
                let auth_header_type = self.auth_header_type_input.clone();
                
                return cosmic::task::future(async move {
                    let result = helpers::fetch_tenants(&url, &token, &auth_header_type).await;
                    cosmic::Action::App(Message::TenantsLoaded(result))
                });
            }

            Message::TenantsLoaded(result) => {
                match result {
                    Ok(tenants) => {
                        self.available_tenants = tenants;
                    }
                    Err(_) => {
                        // Silently fail - tenants list is optional
                        self.available_tenants.clear();
                    }
                }
            }

            Message::SelectTenant(tenant) => {
                self.tenant_input = tenant;
                // Clear databases when tenant changes and fetch new ones
                self.available_databases.clear();
                self.database_input = String::from("default_database");
                return self.update(Message::FetchDatabases);
            }

            Message::SelectDatabase(database) => {
                self.database_input = database;
            }

            Message::TestConnection => {
                self.connection_status = ConnectionStatus::Connecting;
                let url = self.server_url_input.clone();
                let token = self.auth_token_input.clone();
                let auth_header_type = self.auth_header_type_input.clone();
                
                return cosmic::task::future(async move {
                    let result = helpers::test_connection(&url, &token, &auth_header_type).await;
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
                    let result = helpers::fetch_collections(&url, &token, &auth_header_type, &tenant, &database).await;
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
                        let result = helpers::fetch_documents(&url, &token, &auth_header_type, &collection_id, &tenant, &database).await;
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
                    let result = helpers::fetch_server_info(&url, &token, &auth_header_type).await;
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
