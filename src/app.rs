// SPDX-License-Identifier: MPL-2.0

use crate::api::{Collection, Document, ServerInfo};
use crate::config::{Config, ServerConfig};
use crate::fl;
use crate::helpers;
use crate::pages;
use crate::pages::browser::{AddServerForm, AddServerStatus, BrowserData, BrowserDialog, BrowserMsg, BrowserState};
use crate::widgets::miller_columns::MillerMessage;
use cosmic::app::context_drawer;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::{Length, Subscription};
use cosmic::prelude::*;
use cosmic::widget::{self, about::About, icon, menu, nav_bar};
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
    /// Error message when loading tenants fails
    pub tenants_load_error: Option<String>,
    /// Error message when loading databases fails
    pub databases_load_error: Option<String>,
    /// Server names for dropdown (derived from config.servers)
    pub server_names: Vec<String>,
    /// Current page for collections list (0-indexed)
    pub collections_page: usize,
    /// Current page for documents list (0-indexed)
    pub documents_page: usize,
    /// Items per page for pagination
    pub items_per_page: usize,
    /// Total count of documents in selected collection (if known)
    pub documents_total: Option<usize>,
    /// Active notifications to display
    pub notifications: Vec<Notification>,
    /// Counter for generating unique notification IDs
    pub notification_id_counter: u32,

    // === Dialog state ===
    /// Document being viewed in context drawer
    pub selected_document: Option<Document>,
    /// Collection pending deletion (for confirmation dialog)
    pub delete_collection_target: Option<Collection>,
    /// Document pending deletion (for confirmation dialog)
    pub delete_document_target: Option<Document>,
    /// New collection name input
    pub new_collection_name: String,
    /// Whether the new collection dialog is open
    pub show_new_collection_dialog: bool,
    /// Browser page state
    pub browser: BrowserState,
}

/// What's missing during validation
#[derive(Debug, Clone)]
pub struct ValidationMissing {
    pub tenant_exists: bool,
    pub database_exists: bool,
}

#[derive(Debug, Clone, Default)]
pub enum SettingsStatus {
    #[default]
    Idle,
    Validating,
    Saved,
    #[allow(dead_code)]
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

/// Notification level/type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NotificationLevel {
    #[allow(dead_code)]
    Info,
    Success,
    Warning,
    Error,
}

/// A notification message to display to the user
#[derive(Debug, Clone)]
pub struct Notification {
    pub id: u32,
    pub level: NotificationLevel,
    pub title: String,
    pub message: String,
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

    // Pagination
    CollectionsNextPage,
    CollectionsPrevPage,
    DocumentsNextPage,
    DocumentsPrevPage,

    // Document count
    DocumentCountLoaded(Result<usize, String>),

    // Notifications
    AddNotification(NotificationLevel, String, String),
    DismissNotification(u32),
    CopyNotification(u32),

    // Document details
    ShowDocumentDetails(Document),
    CloseDocumentDetails,

    // Collection management
    OpenNewCollectionDialog,
    CloseNewCollectionDialog,
    NewCollectionNameChanged(String),
    CreateCollection,
    CreateCollectionResult(Result<Collection, String>),

    // Delete collection
    RequestDeleteCollection(Collection),
    ConfirmDeleteCollection,
    CancelDeleteCollection,
    DeleteCollectionResult(Result<(), String>),

    // Delete document
    RequestDeleteDocument(Document),
    ConfirmDeleteDocument,
    CancelDeleteDocument,
    DeleteDocumentResult(Result<(), String>),

    // Browser
    Browser(BrowserMsg),
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
        // Create a nav bar with four pages: Browser, Dashboard, Collections and Settings
        let mut nav = nav_bar::Model::default();

        nav.insert()
            .text("Browser")
            .data::<Page>(Page::Browser)
            .icon(icon::from_name("folder-open-symbolic"))
            .activate();

        nav.insert()
            .text(fl!("dashboard"))
            .data::<Page>(Page::Dashboard)
            .icon(icon::from_name("utilities-system-monitor-symbolic"));

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

        // Compute server names for dropdown
        let server_names: Vec<String> = config.servers.iter().map(|s| s.name.clone()).collect();

        // Initialize browser state before config moves
        let browser = BrowserState::new(&config.servers);

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
            tenants_load_error: None,
            databases_load_error: None,
            server_names,
            collections_page: 0,
            documents_page: 0,
            items_per_page: 20,
            documents_total: None,
            notifications: Vec::new(),
            notification_id_counter: 0,
            selected_document: None,
            delete_collection_target: None,
            delete_document_target: None,
            new_collection_name: String::new(),
            show_new_collection_dialog: false,
            browser,
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

        Some(match &self.context_page {
            ContextPage::About => context_drawer::about(
                &self.about,
                |url| Message::LaunchUrl(url.to_string()),
                Message::ToggleContextPage(ContextPage::About.clone()),
            ),
            ContextPage::DocumentDetails => {
                let content =
                    pages::widgets::document_details_view(self.selected_document.as_ref());
                context_drawer::context_drawer(content, Message::CloseDocumentDetails)
                    .title(fl!("document-details"))
            }
        })
    }

    /// Describes the interface based on the current state of the application model.
    fn view(&self) -> Element<'_, Self::Message> {
        let space_s = cosmic::theme::spacing().space_s;
        let space_m = cosmic::theme::spacing().space_m;

        let page_content: Element<_> =
            match self.nav.active_data::<Page>().unwrap_or(&Page::Browser) {
                Page::Browser => {
                    pages::browser::view(&self.browser, Message::Browser, space_s, space_m)
                }
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

        // Build view with notifications at the top if any
        let mut content_column = widget::column::with_capacity(2);

        // Add notifications section if there are any
        if !self.notifications.is_empty() {
            let notifications_row = widget::row::with_children(
                self.notifications
                    .iter()
                    .map(|n| pages::widgets::notification_toast(n)),
            )
            .spacing(space_s);
            content_column = content_column.push(notifications_row);
        }

        content_column = content_column.push(page_content);

        widget::container(content_column)
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
                // Update server names for dropdown
                self.server_names = self.config.servers.iter().map(|s| s.name.clone()).collect();
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
                    // Update server names for dropdown
                    self.server_names =
                        self.config.servers.iter().map(|s| s.name.clone()).collect();
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
                // Update server names for dropdown (name might have changed)
                self.server_names = self.config.servers.iter().map(|s| s.name.clone()).collect();

                if let Some(ref context) = self.config_context {
                    if let Err(e) = self.config.write_entry(context) {
                        eprintln!("Failed to save config: {}", e);
                        self.settings_status =
                            SettingsStatus::Error(format!("Failed to save: {}", e));
                        // Add error notification
                        return self.update(Message::AddNotification(
                            NotificationLevel::Error,
                            fl!("error"),
                            format!("Failed to save: {}", e),
                        ));
                    } else {
                        self.settings_status = SettingsStatus::Saved;
                        // Add success notification
                        return self.update(Message::AddNotification(
                            NotificationLevel::Success,
                            fl!("settings-saved"),
                            String::new(),
                        ));
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
                    let result = helpers::validate_tenant_database(
                        &url,
                        &token,
                        &auth_header_type,
                        &tenant,
                        &database,
                    )
                    .await;
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
                        // Build missing resources message
                        let mut missing_parts = Vec::new();
                        if !tenant_exists {
                            missing_parts.push(format!(
                                "{} '{}'",
                                fl!("tenant"),
                                self.tenant_input
                            ));
                        }
                        if !database_exists {
                            missing_parts.push(format!(
                                "{} '{}'",
                                fl!("database"),
                                self.database_input
                            ));
                        }
                        let missing_msg =
                            format!("{}: {}", fl!("missing-resources"), missing_parts.join(", "));

                        // Show what's missing and offer to create
                        self.settings_status =
                            SettingsStatus::MissingResources(ValidationMissing {
                                tenant_exists,
                                database_exists,
                            });

                        // Add warning notification
                        return self.update(Message::AddNotification(
                            NotificationLevel::Warning,
                            fl!("missing-resources"),
                            missing_msg,
                        ));
                    }
                }
            }

            Message::CreateMissingResources => {
                // Extract the missing info before reassigning settings_status
                let missing_info =
                    if let SettingsStatus::MissingResources(missing) = &self.settings_status {
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
                        let result = helpers::create_missing_resources(
                            &url,
                            &token,
                            &auth_header_type,
                            &tenant,
                            &database,
                            tenant_exists,
                            database_exists,
                        )
                        .await;
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
                        let error_msg = format!("Failed to create resources: {}", e);
                        self.settings_status = SettingsStatus::Error(error_msg.clone());
                        // Add error notification
                        return self.update(Message::AddNotification(
                            NotificationLevel::Error,
                            fl!("error"),
                            error_msg,
                        ));
                    }
                }
            }

            Message::FetchDatabases => {
                let url = self.server_url_input.clone();
                let token = self.auth_token_input.clone();
                let auth_header_type = self.auth_header_type_input.clone();
                let tenant = self.tenant_input.clone();

                return cosmic::task::future(async move {
                    let result =
                        helpers::fetch_databases(&url, &token, &auth_header_type, &tenant).await;
                    cosmic::Action::App(Message::DatabasesLoaded(result))
                });
            }

            Message::DatabasesLoaded(result) => match result {
                Ok(databases) => {
                    self.available_databases = databases;
                    self.databases_load_error = None;
                }
                Err(e) => {
                    self.available_databases.clear();
                    self.databases_load_error = Some(e);
                }
            },

            Message::FetchTenants => {
                let url = self.server_url_input.clone();
                let token = self.auth_token_input.clone();
                let auth_header_type = self.auth_header_type_input.clone();

                eprintln!(
                    "[DEBUG] FetchTenants: url={}, auth_header_type={}",
                    url, auth_header_type
                );

                return cosmic::task::future(async move {
                    eprintln!("[DEBUG] FetchTenants: Starting fetch...");
                    let result = helpers::fetch_tenants(&url, &token, &auth_header_type).await;
                    eprintln!("[DEBUG] FetchTenants: Result = {:?}", result);
                    cosmic::Action::App(Message::TenantsLoaded(result))
                });
            }

            Message::TenantsLoaded(result) => {
                eprintln!("[DEBUG] TenantsLoaded: {:?}", result);
                match result {
                    Ok(tenants) => {
                        eprintln!(
                            "[DEBUG] TenantsLoaded: Found {} tenants: {:?}",
                            tenants.len(),
                            tenants
                        );
                        self.available_tenants = tenants;
                        self.tenants_load_error = None;
                    }
                    Err(e) => {
                        eprintln!("[DEBUG] TenantsLoaded: Error = {}", e);
                        self.available_tenants.clear();
                        self.tenants_load_error = Some(e);
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
                        // Add success notification
                        return self.update(Message::AddNotification(
                            NotificationLevel::Success,
                            fl!("status-connected"),
                            String::new(),
                        ));
                    }
                    Err(e) => {
                        self.connection_status = ConnectionStatus::Error(e.clone());
                        // Add error notification
                        return self.update(Message::AddNotification(
                            NotificationLevel::Error,
                            fl!("status-error"),
                            e,
                        ));
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
                    let result = helpers::fetch_collections(
                        &url,
                        &token,
                        &auth_header_type,
                        &tenant,
                        &database,
                    )
                    .await;
                    cosmic::Action::App(Message::CollectionsLoaded(result))
                });
            }

            Message::CollectionsLoaded(result) => match result {
                Ok(collections) => {
                    self.collections = collections;
                    self.connection_status = ConnectionStatus::Connected;
                }
                Err(e) => {
                    self.connection_status = ConnectionStatus::Error(e);
                }
            },

            Message::SelectCollection(collection) => {
                let collection_id = collection.id.clone();
                self.selected_collection = Some(collection);
                self.documents.clear();
                self.documents_page = 0; // Reset to first page
                self.documents_total = None; // Clear old count

                // Fetch document count
                let active = self.config.active_config();
                let url = active.server_url.clone();
                let token = active.auth_token.clone();
                let auth_header_type = active.auth_header_type.clone();
                let tenant = active.tenant.clone();
                let database = active.database.clone();

                // Spawn count fetch in background
                let count_task = cosmic::task::future(async move {
                    let result = helpers::fetch_document_count(
                        &url,
                        &token,
                        &auth_header_type,
                        &collection_id,
                        &tenant,
                        &database,
                    )
                    .await;
                    cosmic::Action::App(Message::DocumentCountLoaded(result))
                });

                // Also fetch documents
                let docs_task = self.update(Message::FetchDocuments);

                return cosmic::task::batch(vec![count_task, docs_task]);
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
                    let limit = self.items_per_page;
                    let offset = self.documents_page * self.items_per_page;

                    return cosmic::task::future(async move {
                        let result = helpers::fetch_documents(
                            &url,
                            &token,
                            &auth_header_type,
                            &collection_id,
                            &tenant,
                            &database,
                            limit,
                            offset,
                        )
                        .await;
                        cosmic::Action::App(Message::DocumentsLoaded(result))
                    });
                }
            }

            Message::DocumentsLoaded(result) => match result {
                Ok(documents) => {
                    self.documents = documents;
                    self.connection_status = ConnectionStatus::Connected;
                }
                Err(e) => {
                    self.connection_status = ConnectionStatus::Error(e);
                }
            },

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

            // Pagination
            Message::CollectionsNextPage => {
                let total_pages =
                    (self.collections.len() + self.items_per_page - 1) / self.items_per_page;
                if self.collections_page + 1 < total_pages {
                    self.collections_page += 1;
                }
            }

            Message::CollectionsPrevPage => {
                if self.collections_page > 0 {
                    self.collections_page -= 1;
                }
            }

            Message::DocumentsNextPage => {
                self.documents_page += 1;
                // Fetch next page of documents
                return self.update(Message::FetchDocuments);
            }

            Message::DocumentsPrevPage => {
                if self.documents_page > 0 {
                    self.documents_page -= 1;
                    return self.update(Message::FetchDocuments);
                }
            }

            // Document count
            Message::DocumentCountLoaded(result) => match result {
                Ok(count) => {
                    self.documents_total = Some(count);
                }
                Err(e) => {
                    eprintln!("Failed to load document count: {}", e);
                }
            },

            // Notifications
            Message::AddNotification(level, title, message) => {
                self.notification_id_counter += 1;
                self.notifications.push(Notification {
                    id: self.notification_id_counter,
                    level,
                    title,
                    message,
                });
            }

            Message::DismissNotification(id) => {
                self.notifications.retain(|n| n.id != id);
            }

            Message::CopyNotification(id) => {
                if let Some(notification) = self.notifications.iter().find(|n| n.id == id) {
                    let text = format!("{}: {}", notification.title, notification.message);
                    return cosmic::task::future(async move {
                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                            let _ = clipboard.set_text(&text);
                        }
                        cosmic::Action::App(Message::AddNotification(
                            NotificationLevel::Success,
                            fl!("notification-copied"),
                            String::new(),
                        ))
                    });
                }
            }

            // Document details
            Message::ShowDocumentDetails(document) => {
                self.selected_document = Some(document);
                self.context_page = ContextPage::DocumentDetails;
                self.core.window.show_context = true;
            }

            Message::CloseDocumentDetails => {
                self.selected_document = None;
                self.core.window.show_context = false;
            }

            // Collection management
            Message::OpenNewCollectionDialog => {
                self.new_collection_name = String::new();
                self.show_new_collection_dialog = true;
            }

            Message::CloseNewCollectionDialog => {
                self.show_new_collection_dialog = false;
                self.new_collection_name = String::new();
            }

            Message::NewCollectionNameChanged(name) => {
                self.new_collection_name = name;
            }

            Message::CreateCollection => {
                if self.new_collection_name.is_empty() {
                    return Task::none();
                }
                let active = self.config.active_config();
                let url = active.server_url.clone();
                let token = active.auth_token.clone();
                let auth_header_type = active.auth_header_type.clone();
                let tenant = active.tenant.clone();
                let database = active.database.clone();
                let name = self.new_collection_name.clone();

                return cosmic::task::future(async move {
                    let result = helpers::create_collection(
                        &url,
                        &token,
                        &auth_header_type,
                        &name,
                        &tenant,
                        &database,
                    )
                    .await;
                    cosmic::Action::App(Message::CreateCollectionResult(result))
                });
            }

            Message::CreateCollectionResult(result) => {
                self.show_new_collection_dialog = false;
                self.new_collection_name = String::new();
                match result {
                    Ok(collection) => {
                        // Add success notification inline
                        self.notification_id_counter += 1;
                        self.notifications.push(Notification {
                            id: self.notification_id_counter,
                            level: NotificationLevel::Success,
                            title: fl!("collection-created"),
                            message: format!("Collection '{}' created", collection.name),
                        });
                        // Refresh collections list
                        return self.update(Message::FetchCollections);
                    }
                    Err(e) => {
                        return self.update(Message::AddNotification(
                            NotificationLevel::Error,
                            fl!("error"),
                            e,
                        ));
                    }
                }
            }

            // Delete collection
            Message::RequestDeleteCollection(collection) => {
                self.delete_collection_target = Some(collection);
            }

            Message::ConfirmDeleteCollection => {
                if let Some(ref collection) = self.delete_collection_target {
                    let active = self.config.active_config();
                    let url = active.server_url.clone();
                    let token = active.auth_token.clone();
                    let auth_header_type = active.auth_header_type.clone();
                    let tenant = active.tenant.clone();
                    let database = active.database.clone();
                    let collection_name = collection.name.clone();

                    return cosmic::task::future(async move {
                        let result = helpers::delete_collection(
                            &url,
                            &token,
                            &auth_header_type,
                            &collection_name,
                            &tenant,
                            &database,
                        )
                        .await;
                        cosmic::Action::App(Message::DeleteCollectionResult(result))
                    });
                }
            }

            Message::CancelDeleteCollection => {
                self.delete_collection_target = None;
            }

            Message::DeleteCollectionResult(result) => {
                let deleted_name = self
                    .delete_collection_target
                    .as_ref()
                    .map(|c| c.name.clone());
                self.delete_collection_target = None;
                match result {
                    Ok(()) => {
                        if let Some(name) = deleted_name {
                            // Add success notification inline
                            self.notification_id_counter += 1;
                            self.notifications.push(Notification {
                                id: self.notification_id_counter,
                                level: NotificationLevel::Success,
                                title: fl!("collection-deleted"),
                                message: format!("Collection '{}' deleted", name),
                            });
                        }
                        // Refresh collections list
                        return self.update(Message::FetchCollections);
                    }
                    Err(e) => {
                        return self.update(Message::AddNotification(
                            NotificationLevel::Error,
                            fl!("error"),
                            e,
                        ));
                    }
                }
            }

            // Delete document
            Message::RequestDeleteDocument(document) => {
                self.delete_document_target = Some(document);
            }

            Message::ConfirmDeleteDocument => {
                if let Some(ref document) = self.delete_document_target {
                    if let Some(ref collection) = self.selected_collection {
                        let active = self.config.active_config();
                        let url = active.server_url.clone();
                        let token = active.auth_token.clone();
                        let auth_header_type = active.auth_header_type.clone();
                        let tenant = active.tenant.clone();
                        let database = active.database.clone();
                        let collection_id = collection.id.clone();
                        let document_id = document.id.clone();

                        return cosmic::task::future(async move {
                            let result = helpers::delete_document(
                                &url,
                                &token,
                                &auth_header_type,
                                &collection_id,
                                &document_id,
                                &tenant,
                                &database,
                            )
                            .await;
                            cosmic::Action::App(Message::DeleteDocumentResult(result))
                        });
                    }
                }
            }

            Message::CancelDeleteDocument => {
                self.delete_document_target = None;
            }

            Message::DeleteDocumentResult(result) => {
                let deleted_id = self.delete_document_target.as_ref().map(|d| d.id.clone());
                self.delete_document_target = None;
                match result {
                    Ok(()) => {
                        if let Some(id) = deleted_id {
                            // Add success notification inline
                            self.notification_id_counter += 1;
                            self.notifications.push(Notification {
                                id: self.notification_id_counter,
                                level: NotificationLevel::Success,
                                title: fl!("document-deleted"),
                                message: format!("Document '{}' deleted", id),
                            });
                        }
                        // Refresh documents list
                        return self.update(Message::FetchDocuments);
                    }
                    Err(e) => {
                        return self.update(Message::AddNotification(
                            NotificationLevel::Error,
                            fl!("error"),
                            e,
                        ));
                    }
                }
            }

            // Browser messages
            Message::Browser(browser_msg) => {
                return self.handle_browser_message(browser_msg);
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

    /// Handles browser messages.
    fn handle_browser_message(&mut self, msg: BrowserMsg) -> Task<cosmic::Action<Message>> {
        match msg {
            BrowserMsg::Miller(miller_msg) => {
                match miller_msg {
                    MillerMessage::Select { column: _, path, item } => {
                        // Update selection in miller state using full path
                        self.browser.miller.select(path);

                        // Clear document preview when selecting non-document items
                        if !matches!(&item.data, BrowserData::Document { .. }) {
                            self.browser.selected_document = None;
                        }

                        // If it's a branch item, we need to load children
                        match &item.data {
                            BrowserData::Server { index, config } => {
                                // Load tenants for this server
                                let server_index = *index;
                                self.browser.set_tenants_loading(server_index);
                                let url = config.server_url.clone();
                                let token = config.auth_token.clone();
                                let auth_header_type = config.auth_header_type.clone();

                                return cosmic::task::future(async move {
                                    let result =
                                        helpers::fetch_tenants(&url, &token, &auth_header_type)
                                            .await;
                                    cosmic::Action::App(Message::Browser(BrowserMsg::TenantsLoaded {
                                        server_index,
                                        result,
                                    }))
                                });
                            }
                            BrowserData::Tenant { server_index, name } => {
                                // Load databases for this tenant
                                let server_index = *server_index;
                                let tenant = name.clone();
                                self.browser.set_databases_loading(server_index, &tenant);

                                let config = &self.config.servers[server_index];
                                let url = config.server_url.clone();
                                let token = config.auth_token.clone();
                                let auth_header_type = config.auth_header_type.clone();

                                return cosmic::task::future(async move {
                                    let result = helpers::fetch_databases(
                                        &url,
                                        &token,
                                        &auth_header_type,
                                        &tenant,
                                    )
                                    .await;
                                    cosmic::Action::App(Message::Browser(
                                        BrowserMsg::DatabasesLoaded {
                                            server_index,
                                            tenant,
                                            result,
                                        },
                                    ))
                                });
                            }
                            BrowserData::Database {
                                server_index,
                                tenant,
                                name,
                            } => {
                                // Load collections for this database
                                let server_index = *server_index;
                                let tenant = tenant.clone();
                                let database = name.clone();
                                self.browser
                                    .set_collections_loading(server_index, &tenant, &database);

                                let config = &self.config.servers[server_index];
                                let url = config.server_url.clone();
                                let token = config.auth_token.clone();
                                let auth_header_type = config.auth_header_type.clone();

                                return cosmic::task::future(async move {
                                    let result = helpers::fetch_collections(
                                        &url,
                                        &token,
                                        &auth_header_type,
                                        &tenant,
                                        &database,
                                    )
                                    .await;
                                    cosmic::Action::App(Message::Browser(
                                        BrowserMsg::CollectionsLoaded {
                                            server_index,
                                            tenant,
                                            database,
                                            result,
                                        },
                                    ))
                                });
                            }
                            BrowserData::Collection {
                                server_index,
                                tenant,
                                database,
                                collection,
                            } => {
                                // Load documents for this collection
                                let server_index = *server_index;
                                let tenant = tenant.clone();
                                let database = database.clone();
                                let collection_id = collection.id.clone();
                                self.browser.set_documents_loading(
                                    server_index,
                                    &tenant,
                                    &database,
                                    &collection_id,
                                );

                                let config = &self.config.servers[server_index];
                                let url = config.server_url.clone();
                                let token = config.auth_token.clone();
                                let auth_header_type = config.auth_header_type.clone();

                                return cosmic::task::future(async move {
                                    let result = helpers::fetch_documents(
                                        &url,
                                        &token,
                                        &auth_header_type,
                                        &collection_id,
                                        &tenant,
                                        &database,
                                        100, // limit
                                        0,   // offset
                                    )
                                    .await;
                                    cosmic::Action::App(Message::Browser(
                                        BrowserMsg::DocumentsLoaded {
                                            server_index,
                                            tenant,
                                            database,
                                            collection_id,
                                            result,
                                        },
                                    ))
                                });
                            }
                            BrowserData::Document { document, .. } => {
                                // Show document preview
                                self.browser.selected_document = Some(document.clone());
                            }
                            _ => {}
                        }
                    }
                    MillerMessage::Activate { item, .. } => {
                        // Handle activation (e.g., click on leaf items like "Add" buttons)
                        match &item.data {
                            BrowserData::AddServer => {
                                // Show inline form instead of dialog
                                self.browser.adding_server = Some(AddServerForm::new());
                            }
                            BrowserData::AddTenant { server_index } => {
                                self.browser.dialog = Some(BrowserDialog::AddTenant {
                                    server_index: *server_index,
                                    name: String::new(),
                                });
                            }
                            BrowserData::AddDatabase {
                                server_index,
                                tenant,
                            } => {
                                self.browser.dialog = Some(BrowserDialog::AddDatabase {
                                    server_index: *server_index,
                                    tenant: tenant.clone(),
                                    name: String::new(),
                                });
                            }
                            BrowserData::AddCollection {
                                server_index,
                                tenant,
                                database,
                            } => {
                                self.browser.dialog = Some(BrowserDialog::AddCollection {
                                    server_index: *server_index,
                                    tenant: tenant.clone(),
                                    database: database.clone(),
                                    name: String::new(),
                                });
                            }
                            BrowserData::Document { document, .. } => {
                                // Show document preview
                                self.browser.selected_document = Some(document.clone());
                            }
                            _ => {}
                        }
                    }
                    MillerMessage::NeedChildren { .. } => {
                        // This is handled by Select above
                    }
                    MillerMessage::Scroll { .. } => {
                        // Handle scroll if needed
                    }
                }
            }

            BrowserMsg::TenantsLoaded {
                server_index,
                result,
            } => match result {
                Ok(mut tenants) => {
                    // Merge with tenants from config (locally added tenants that may not exist on server yet)
                    if server_index < self.config.servers.len() {
                        for config_tenant in &self.config.servers[server_index].tenants {
                            if !tenants.contains(config_tenant) {
                                tenants.push(config_tenant.clone());
                            }
                        }
                    }
                    self.browser.set_tenants(server_index, tenants);
                }
                Err(e) => {
                    // Even on error, show tenants from config if available
                    if server_index < self.config.servers.len() {
                        let config_tenants = self.config.servers[server_index].tenants.clone();
                        if !config_tenants.is_empty() {
                            self.browser.set_tenants(server_index, config_tenants);
                            return Task::none();
                        }
                    }
                    self.browser.set_tenants_error(server_index, e);
                }
            },

            BrowserMsg::DatabasesLoaded {
                server_index,
                tenant,
                result,
            } => match result {
                Ok(databases) => {
                    self.browser.set_databases(server_index, &tenant, databases);
                }
                Err(e) => {
                    // Check if the error indicates tenant doesn't exist
                    let tenant_not_found = e.to_lowercase().contains("tenant")
                        && (e.to_lowercase().contains("not found")
                            || e.to_lowercase().contains("does not exist")
                            || e.contains("404"));

                    if tenant_not_found {
                        // Show confirmation dialog to create tenant on server
                        self.browser.dialog = Some(BrowserDialog::ConfirmCreateTenant {
                            server_index,
                            tenant: tenant.clone(),
                        });
                    } else {
                        self.browser.set_databases_error(server_index, &tenant, e);
                    }
                }
            },

            BrowserMsg::CollectionsLoaded {
                server_index,
                tenant,
                database,
                result,
            } => match result {
                Ok(collections) => {
                    self.browser
                        .set_collections(server_index, &tenant, &database, collections);
                }
                Err(e) => {
                    self.browser
                        .set_collections_error(server_index, &tenant, &database, e);
                }
            },

            BrowserMsg::DocumentsLoaded {
                server_index,
                tenant,
                database,
                collection_id,
                result,
            } => match result {
                Ok(documents) => {
                    self.browser.set_documents(
                        server_index,
                        &tenant,
                        &database,
                        &collection_id,
                        documents,
                    );
                }
                Err(e) => {
                    self.browser
                        .set_documents_error(server_index, &tenant, &database, &collection_id, e);
                }
            },

            BrowserMsg::DialogInputChanged(value) => {
                if let Some(ref mut dialog) = self.browser.dialog {
                    match dialog {
                        BrowserDialog::AddServer { name } => *name = value,
                        BrowserDialog::AddTenant { name, .. } => *name = value,
                        BrowserDialog::AddDatabase { name, .. } => *name = value,
                        BrowserDialog::AddCollection { name, .. } => *name = value,
                        // ConfirmCreateTenant has no text input
                        BrowserDialog::ConfirmCreateTenant { .. } => {}
                    }
                }
            }

            BrowserMsg::DialogCancel => {
                self.browser.dialog = None;
            }

            BrowserMsg::DialogConfirm => {
                if let Some(dialog) = self.browser.dialog.take() {
                    match dialog {
                        BrowserDialog::AddServer { name } => {
                            // Add new server to config
                            let new_server = ServerConfig::new(&name);
                            self.config.add_server(new_server);
                            if let Some(ref context) = self.config_context {
                                let _ = self.config.write_entry(context);
                            }
                            // Refresh browser with new servers
                            self.browser.refresh_servers(&self.config.servers);
                            self.server_names =
                                self.config.servers.iter().map(|s| s.name.clone()).collect();
                        }
                        BrowserDialog::AddTenant { server_index, name } => {
                            // Add tenant to local cache and persist to config
                            let mut tenants = self
                                .browser
                                .tenants_cache
                                .get(&server_index)
                                .cloned()
                                .unwrap_or_default();
                            if !tenants.contains(&name) {
                                tenants.push(name.clone());
                                self.browser.set_tenants(server_index, tenants);

                                // Also save to config for persistence
                                if server_index < self.config.servers.len() {
                                    self.config.servers[server_index].add_tenant(&name);
                                    if let Some(ref context) = self.config_context {
                                        let _ = self.config.write_entry(context);
                                    }
                                }
                            }
                        }
                        BrowserDialog::ConfirmCreateTenant {
                            server_index,
                            tenant,
                        } => {
                            // User confirmed creating tenant on server
                            let config = &self.config.servers[server_index];
                            let url = config.server_url.clone();
                            let token = config.auth_token.clone();
                            let auth_header_type = config.auth_header_type.clone();

                            return cosmic::task::future(async move {
                                let result =
                                    helpers::create_tenant(&url, &token, &auth_header_type, &tenant)
                                        .await;
                                cosmic::Action::App(Message::Browser(BrowserMsg::TenantCreated {
                                    server_index,
                                    tenant,
                                    result,
                                }))
                            });
                        }
                        BrowserDialog::AddDatabase {
                            server_index,
                            tenant,
                            name,
                        } => {
                            // Create database
                            let config = &self.config.servers[server_index];
                            let url = config.server_url.clone();
                            let token = config.auth_token.clone();
                            let auth_header_type = config.auth_header_type.clone();

                            return cosmic::task::future(async move {
                                let result = helpers::create_database(
                                    &url,
                                    &token,
                                    &auth_header_type,
                                    &name,
                                    &tenant,
                                )
                                .await;
                                cosmic::Action::App(Message::Browser(BrowserMsg::DatabaseCreated {
                                    server_index,
                                    tenant,
                                    database: name,
                                    result,
                                }))
                            });
                        }
                        BrowserDialog::AddCollection {
                            server_index,
                            tenant,
                            database,
                            name,
                        } => {
                            // Create collection
                            let config = &self.config.servers[server_index];
                            let url = config.server_url.clone();
                            let token = config.auth_token.clone();
                            let auth_header_type = config.auth_header_type.clone();

                            return cosmic::task::future(async move {
                                let result = helpers::create_collection(
                                    &url,
                                    &token,
                                    &auth_header_type,
                                    &name,
                                    &tenant,
                                    &database,
                                )
                                .await;
                                cosmic::Action::App(Message::Browser(
                                    BrowserMsg::CollectionCreated {
                                        server_index,
                                        tenant,
                                        database,
                                        result,
                                    },
                                ))
                            });
                        }
                    }
                }
            }

            BrowserMsg::ServerCreated => {
                // Refresh servers
                self.browser.refresh_servers(&self.config.servers);
            }

            BrowserMsg::TenantCreated {
                server_index,
                tenant,
                result,
            } => match result {
                Ok(()) => {
                    // Tenant created successfully - now load databases for this tenant
                    self.browser.set_databases_loading(server_index, &tenant);
                    let config = &self.config.servers[server_index];
                    let url = config.server_url.clone();
                    let token = config.auth_token.clone();
                    let auth_header_type = config.auth_header_type.clone();

                    // Show success notification
                    self.notification_id_counter += 1;
                    self.notifications.push(Notification {
                        id: self.notification_id_counter,
                        level: NotificationLevel::Success,
                        title: "Tenant created".to_string(),
                        message: format!("Tenant '{}' created on server", tenant),
                    });

                    return cosmic::task::future(async move {
                        let result =
                            helpers::fetch_databases(&url, &token, &auth_header_type, &tenant)
                                .await;
                        cosmic::Action::App(Message::Browser(BrowserMsg::DatabasesLoaded {
                            server_index,
                            tenant,
                            result,
                        }))
                    });
                }
                Err(e) => {
                    self.notification_id_counter += 1;
                    self.notifications.push(Notification {
                        id: self.notification_id_counter,
                        level: NotificationLevel::Error,
                        title: format!("Failed to create tenant '{}'", tenant),
                        message: e,
                    });
                }
            },

            BrowserMsg::DatabaseCreated {
                server_index,
                tenant,
                database,
                result,
            } => match result {
                Ok(()) => {
                    // Refresh databases for this tenant
                    self.browser.set_databases_loading(server_index, &tenant);
                    let config = &self.config.servers[server_index];
                    let url = config.server_url.clone();
                    let token = config.auth_token.clone();
                    let auth_header_type = config.auth_header_type.clone();

                    return cosmic::task::future(async move {
                        let result =
                            helpers::fetch_databases(&url, &token, &auth_header_type, &tenant)
                                .await;
                        cosmic::Action::App(Message::Browser(BrowserMsg::DatabasesLoaded {
                            server_index,
                            tenant,
                            result,
                        }))
                    });
                }
                Err(e) => {
                    self.notification_id_counter += 1;
                    self.notifications.push(Notification {
                        id: self.notification_id_counter,
                        level: NotificationLevel::Error,
                        title: format!("Failed to create database '{}'", database),
                        message: e,
                    });
                }
            },

            BrowserMsg::CollectionCreated {
                server_index,
                tenant,
                database,
                result,
            } => match result {
                Ok(_collection) => {
                    // Refresh collections for this database
                    self.browser
                        .set_collections_loading(server_index, &tenant, &database);
                    let config = &self.config.servers[server_index];
                    let url = config.server_url.clone();
                    let token = config.auth_token.clone();
                    let auth_header_type = config.auth_header_type.clone();

                    return cosmic::task::future(async move {
                        let result = helpers::fetch_collections(
                            &url,
                            &token,
                            &auth_header_type,
                            &tenant,
                            &database,
                        )
                        .await;
                        cosmic::Action::App(Message::Browser(BrowserMsg::CollectionsLoaded {
                            server_index,
                            tenant,
                            database,
                            result,
                        }))
                    });
                }
                Err(e) => {
                    self.notification_id_counter += 1;
                    self.notifications.push(Notification {
                        id: self.notification_id_counter,
                        level: NotificationLevel::Error,
                        title: "Failed to create collection".to_string(),
                        message: e,
                    });
                }
            },

            // Add Server form handlers
            BrowserMsg::StartAddServer => {
                self.browser.adding_server = Some(AddServerForm::new());
            }

            BrowserMsg::CancelAddServer => {
                self.browser.adding_server = None;
            }

            BrowserMsg::AddServerNameChanged(name) => {
                if let Some(ref mut form) = self.browser.adding_server {
                    form.name = name;
                }
            }

            BrowserMsg::AddServerUrlChanged(url) => {
                if let Some(ref mut form) = self.browser.adding_server {
                    form.url = url;
                    // Reset status when URL changes
                    form.status = AddServerStatus::Editing;
                }
            }

            BrowserMsg::AddServerAuthTokenChanged(token) => {
                if let Some(ref mut form) = self.browser.adding_server {
                    form.auth_token = token;
                    // Reset status when auth changes
                    form.status = AddServerStatus::Editing;
                }
            }

            BrowserMsg::AddServerAuthHeaderTypeChanged(auth_type) => {
                if let Some(ref mut form) = self.browser.adding_server {
                    form.auth_header_type = auth_type;
                    // Reset status when auth type changes
                    form.status = AddServerStatus::Editing;
                }
            }

            BrowserMsg::TestNewServerConnection => {
                if let Some(ref mut form) = self.browser.adding_server {
                    form.status = AddServerStatus::Testing;
                    let url = form.url.clone();
                    let token = form.auth_token.clone();
                    let auth_header_type = form.auth_header_type.clone();

                    return cosmic::task::future(async move {
                        let result =
                            helpers::test_connection(&url, &token, &auth_header_type).await;
                        cosmic::Action::App(Message::Browser(BrowserMsg::TestNewServerResult(
                            result,
                        )))
                    });
                }
            }

            BrowserMsg::TestNewServerResult(result) => {
                if let Some(ref mut form) = self.browser.adding_server {
                    form.status = match result {
                        Ok(()) => AddServerStatus::TestSuccess,
                        Err(e) => AddServerStatus::TestFailed(e),
                    };
                }
            }

            BrowserMsg::SaveNewServer => {
                if let Some(form) = self.browser.adding_server.take() {
                    // Only save if test was successful
                    if form.status == AddServerStatus::TestSuccess && !form.name.is_empty() {
                        let mut new_server = ServerConfig::new(&form.name);
                        new_server.server_url = form.url;
                        new_server.auth_token = form.auth_token;
                        new_server.auth_header_type = form.auth_header_type;

                        self.config.add_server(new_server);
                        if let Some(ref context) = self.config_context {
                            let _ = self.config.write_entry(context);
                        }

                        // Refresh browser with new servers
                        self.browser.refresh_servers(&self.config.servers);
                        self.server_names =
                            self.config.servers.iter().map(|s| s.name.clone()).collect();

                        self.notification_id_counter += 1;
                        self.notifications.push(Notification {
                            id: self.notification_id_counter,
                            level: NotificationLevel::Success,
                            title: "Server added".to_string(),
                            message: "New server has been added successfully.".to_string(),
                        });
                    }
                }
            }
        }

        Task::none()
    }
}

/// The page to display in the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    Browser,
    Dashboard,
    Collections,
    Settings,
}

/// The context page to display in the context drawer.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum ContextPage {
    #[default]
    About,
    DocumentDetails,
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
