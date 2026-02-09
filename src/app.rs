// SPDX-License-Identifier: MPL-2.0

use crate::api::{ChromaClient, Collection};
use crate::config::Config;
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
    /// Temporary server URL input (before saving)
    server_url_input: String,
    /// Temporary auth token input (before saving)
    auth_token_input: String,
    /// Temporary auth header type input (before saving)
    auth_header_type_input: String,
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
    ServerUrlChanged(String),
    AuthTokenChanged(String),
    AuthHeaderTypeChanged(String),
    SaveSettings,
    
    // Connection & data
    TestConnection,
    ConnectionResult(Result<(), String>),
    FetchCollections,
    CollectionsLoaded(Result<Vec<Collection>, String>),
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
        // Create a nav bar with two pages: Collections and Settings
        let mut nav = nav_bar::Model::default();

        nav.insert()
            .text(fl!("collections"))
            .data::<Page>(Page::Collections)
            .icon(icon::from_name("folder-symbolic"))
            .activate();

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

        // Construct the app model with the runtime's core.
        let mut app = AppModel {
            core,
            context_page: ContextPage::default(),
            about,
            nav,
            key_binds: HashMap::new(),
            server_url_input: config.server_url.clone(),
            auth_token_input: config.auth_token.clone(),
            auth_header_type_input: config.auth_header_type.clone(),
            config,
            config_context,
            collections: Vec::new(),
            connection_status: ConnectionStatus::Disconnected,
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
        
        let content: Element<_> = match self.nav.active_data::<Page>().unwrap_or(&Page::Collections) {
            Page::Collections => self.view_collections(space_s, space_m),
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
                self.server_url_input = self.config.server_url.clone();
                self.auth_token_input = self.config.auth_token.clone();
                self.auth_header_type_input = self.config.auth_header_type.clone();
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

            Message::SaveSettings => {
                self.config.server_url = self.server_url_input.clone();
                self.config.auth_token = self.auth_token_input.clone();
                self.config.auth_header_type = self.auth_header_type_input.clone();
                
                if let Some(ref context) = self.config_context {
                    if let Err(e) = self.config.write_entry(context) {
                        eprintln!("Failed to save config: {}", e);
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
                let url = self.config.server_url.clone();
                let token = self.config.auth_token.clone();
                let auth_header_type = self.config.auth_header_type.clone();
                
                return cosmic::task::future(async move {
                    let result = fetch_collections(&url, &token, &auth_header_type).await;
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
                let item = widget::container(
                    widget::column::with_capacity(2)
                        .push(widget::text::title4(&collection.name))
                        .push(widget::text::caption(format!("ID: {}", collection.id)))
                        .spacing(4)
                )
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

        // Server configuration section
        let server_section = cosmic::widget::settings::section()
            .title(fl!("server-config"))
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
            );

        // Connection status and buttons
        let status_text = match &self.connection_status {
            ConnectionStatus::Disconnected => fl!("status-disconnected"),
            ConnectionStatus::Connecting => fl!("status-connecting"),
            ConnectionStatus::Connected => fl!("status-connected"),
            ConnectionStatus::Error(e) => format!("{}: {}", fl!("status-error"), e),
        };

        let buttons = widget::row::with_capacity(3)
            .push(widget::button::standard(fl!("save")).on_press(Message::SaveSettings))
            .push(widget::button::suggested(fl!("test-connection")).on_press(Message::TestConnection))
            .push(widget::text::body(status_text))
            .spacing(space_s)
            .align_y(Alignment::Center);

        widget::column::with_capacity(3)
            .push(header)
            .push(server_section)
            .push(buttons)
            .spacing(space_m)
            .width(Length::Fill)
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
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

async fn test_connection(url: &str, token: &str, auth_header_type: &str) -> Result<(), String> {
    let client = ChromaClient::new(url, token, auth_header_type).map_err(|e| e.to_string())?;
    client.heartbeat().await.map_err(|e| e.to_string())?;
    Ok(())
}

async fn fetch_collections(url: &str, token: &str, auth_header_type: &str) -> Result<Vec<Collection>, String> {
    let client = ChromaClient::new(url, token, auth_header_type).map_err(|e| e.to_string())?;
    client.list_collections().await.map_err(|e| e.to_string())
}
