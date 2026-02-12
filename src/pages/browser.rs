// SPDX-License-Identifier: MPL-2.0

//! Browser page using column_view for hierarchical navigation.
//!
//! Column hierarchy:
//! 1. Server configs (+ Add New inline form)
//! 2. Tenants (+ Add New)
//! 3. Databases (+ Add New)
//! 4. Collections (+ Add New)
//! 5. Documents
//!
//! Each column is a fully custom Element built on the app side, with
//! the column_view widget from cosmetics handling horizontal layout,
//! scrolling, and resize handles.

use crate::api::{Collection, Document};
use crate::config::ServerConfig;
use cosmetics::widgets::column_view::{column_view, ColumnViewMessage};
use cosmic::iced::{Alignment, Length};
use cosmic::iced_widget::scrollable::{snap_to, RelativeOffset};
use cosmic::prelude::*;
use cosmic::widget::{self, icon, popover, Id};
use std::collections::HashMap;

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Truncate a string to at most `max_chars` characters, appending "..." if truncated.
/// Safe for multi-byte UTF-8 characters (never slices mid-codepoint).
fn truncate_str(s: &str, max_chars: usize) -> String {
    match s.char_indices().nth(max_chars) {
        Some((byte_idx, _)) => format!("{}...", &s[..byte_idx]),
        None => s.to_string(),
    }
}

// ── Constants ────────────────────────────────────────────────────────────────

/// Default column widths per hierarchy level (in pixels).
const SERVER_COLUMN_WIDTH: f32 = 220.0;
const TENANT_COLUMN_WIDTH: f32 = 200.0;
const DATABASE_COLUMN_WIDTH: f32 = 200.0;
const COLLECTION_COLUMN_WIDTH: f32 = 240.0;
const DOCUMENT_COLUMN_WIDTH: f32 = 320.0;

/// Number of hierarchy levels (servers, tenants, databases, collections, documents).
const NUM_COLUMNS: usize = 5;

// ── Load status for async data ───────────────────────────────────────────────

/// Loading status for each column's data.
#[derive(Debug, Clone, Default)]
pub enum LoadStatus {
    #[default]
    Idle,
    Loading,
    Loaded,
    Error(String),
}

// ── Browser data types ───────────────────────────────────────────────────────

/// Form state for adding/editing a server.
#[derive(Debug, Clone, Default)]
pub struct AddServerForm {
    pub name: String,
    pub url: String,
    pub auth_token: String,
    pub auth_header_type: String,
    pub status: AddServerStatus,
}

/// Status of the add server form.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum AddServerStatus {
    #[default]
    Editing,
    Testing,
    TestSuccess,
    TestFailed(String),
}

impl AddServerForm {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            url: String::from("http://localhost:8000"),
            auth_token: String::new(),
            auth_header_type: String::from("authorization"),
            status: AddServerStatus::Editing,
        }
    }
}

/// Unique identifier for items that can have a context menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuItemId {
    Server {
        index: usize,
    },
    Database {
        server_index: usize,
        tenant: String,
        name: String,
    },
    Collection {
        server_index: usize,
        tenant: String,
        database: String,
        collection_id: String,
    },
    Document {
        server_index: usize,
        tenant: String,
        database: String,
        collection_id: String,
        document_id: String,
    },
}

/// Dialog types for adding new items.
#[derive(Debug, Clone)]
pub enum BrowserDialog {
    #[allow(dead_code)]
    AddServer {
        name: String,
    },
    AddTenant {
        server_index: usize,
        name: String,
    },
    AddDatabase {
        server_index: usize,
        tenant: String,
        name: String,
    },
    AddCollection {
        server_index: usize,
        tenant: String,
        database: String,
        name: String,
    },
    /// Confirmation dialog when tenant doesn't exist on server.
    ConfirmCreateTenant {
        server_index: usize,
        tenant: String,
    },
}

// ── Browser state ────────────────────────────────────────────────────────────

/// State for the browser page — owns all hierarchy and UI data.
#[derive(Debug, Clone)]
pub struct BrowserState {
    // Selection state: each level is `Some` when an item is selected.
    pub selected_server: Option<usize>,
    pub selected_tenant: Option<String>,
    pub selected_database: Option<String>,
    pub selected_collection: Option<String>, // collection ID

    // Data caches
    pub tenants_cache: HashMap<usize, Vec<String>>,
    pub databases_cache: HashMap<String, Vec<String>>,
    pub collections_cache: HashMap<String, Vec<Collection>>,
    pub documents_cache: HashMap<String, Vec<Document>>,

    // Load status per level
    pub tenants_status: LoadStatus,
    pub databases_status: LoadStatus,
    pub collections_status: LoadStatus,
    pub documents_status: LoadStatus,

    /// Currently selected document for context drawer preview
    pub selected_document: Option<Document>,

    /// Column widths — one per hierarchy level, persists resize
    pub column_widths: Vec<Option<f32>>,

    /// Dialog state for adding new items
    pub dialog: Option<BrowserDialog>,
    /// Add server form (shown inline in the server column)
    pub adding_server: Option<AddServerForm>,
    /// Edit server form (server_index, form state)
    pub editing_server: Option<(usize, AddServerForm)>,

    /// Scrollable ID for the column_view
    pub scrollable_id: Id,

    /// ID of the item whose context menu is open
    pub open_menu: Option<MenuItemId>,

    /// Server column filter
    pub server_filter: String,
    pub server_filter_visible: bool,
}

impl Default for BrowserState {
    fn default() -> Self {
        Self {
            selected_server: None,
            selected_tenant: None,
            selected_database: None,
            selected_collection: None,
            tenants_cache: HashMap::new(),
            databases_cache: HashMap::new(),
            collections_cache: HashMap::new(),
            documents_cache: HashMap::new(),
            tenants_status: LoadStatus::Idle,
            databases_status: LoadStatus::Idle,
            collections_status: LoadStatus::Idle,
            documents_status: LoadStatus::Idle,
            selected_document: None,
            column_widths: vec![None; NUM_COLUMNS],
            dialog: None,
            adding_server: None,
            editing_server: None,
            scrollable_id: Id::unique(),
            open_menu: None,
            server_filter: String::new(),
            server_filter_visible: false,
        }
    }
}

impl BrowserState {
    /// Creates new browser state from server configs.
    pub fn new(servers: &[ServerConfig]) -> Self {
        // Pre-populate tenants cache from config
        let mut tenants_cache = HashMap::new();
        for (index, config) in servers.iter().enumerate() {
            if !config.tenants.is_empty() {
                tenants_cache.insert(index, config.tenants.clone());
            }
        }

        Self {
            tenants_cache,
            ..Default::default()
        }
    }

    /// Returns a task that scrolls the column_view to the right.
    pub fn scroll_to_end<T>(&self) -> cosmic::Task<T> {
        snap_to(
            self.scrollable_id.clone(),
            RelativeOffset { x: 1.0, y: 0.0 },
        )
    }

    /// Resets selection (called when server list changes).
    pub fn refresh_servers(&mut self, _servers: &[ServerConfig]) {
        self.selected_server = None;
        self.selected_tenant = None;
        self.selected_database = None;
        self.selected_collection = None;
        self.selected_document = None;
        self.tenants_status = LoadStatus::Idle;
        self.databases_status = LoadStatus::Idle;
        self.collections_status = LoadStatus::Idle;
        self.documents_status = LoadStatus::Idle;
    }

    // ── Tenants ──────────────────────────────────────────────────────────

    pub fn set_tenants(&mut self, server_index: usize, tenants: Vec<String>) {
        self.tenants_cache.insert(server_index, tenants);
        self.tenants_status = LoadStatus::Loaded;
    }

    pub fn set_tenants_loading(&mut self, _server_index: usize) {
        self.tenants_status = LoadStatus::Loading;
    }

    pub fn set_tenants_error(&mut self, _server_index: usize, error: String) {
        self.tenants_status = LoadStatus::Error(error);
    }

    // ── Databases ────────────────────────────────────────────────────────

    pub fn set_databases(&mut self, server_index: usize, tenant: &str, databases: Vec<String>) {
        let key = format!("{}:{}", server_index, tenant);
        self.databases_cache.insert(key, databases);
        self.databases_status = LoadStatus::Loaded;
    }

    pub fn set_databases_loading(&mut self, _server_index: usize, _tenant: &str) {
        self.databases_status = LoadStatus::Loading;
    }

    pub fn set_databases_error(&mut self, _server_index: usize, _tenant: &str, error: String) {
        self.databases_status = LoadStatus::Error(error);
    }

    // ── Collections ──────────────────────────────────────────────────────

    pub fn set_collections(
        &mut self,
        server_index: usize,
        tenant: &str,
        database: &str,
        collections: Vec<Collection>,
    ) {
        let key = format!("{}:{}:{}", server_index, tenant, database);
        self.collections_cache.insert(key, collections);
        self.collections_status = LoadStatus::Loaded;
    }

    pub fn set_collections_loading(
        &mut self,
        _server_index: usize,
        _tenant: &str,
        _database: &str,
    ) {
        self.collections_status = LoadStatus::Loading;
    }

    pub fn set_collections_error(
        &mut self,
        _server_index: usize,
        _tenant: &str,
        _database: &str,
        error: String,
    ) {
        self.collections_status = LoadStatus::Error(error);
    }

    // ── Documents ────────────────────────────────────────────────────────

    pub fn set_documents(
        &mut self,
        server_index: usize,
        tenant: &str,
        database: &str,
        collection_id: &str,
        documents: Vec<Document>,
    ) {
        let key = format!("{}:{}:{}:{}", server_index, tenant, database, collection_id);
        self.documents_cache.insert(key, documents);
        self.documents_status = LoadStatus::Loaded;
    }

    pub fn set_documents_loading(
        &mut self,
        _server_index: usize,
        _tenant: &str,
        _database: &str,
        _collection_id: &str,
    ) {
        self.documents_status = LoadStatus::Loading;
    }

    pub fn set_documents_error(
        &mut self,
        _server_index: usize,
        _tenant: &str,
        _database: &str,
        _collection_id: &str,
        error: String,
    ) {
        self.documents_status = LoadStatus::Error(error);
    }

    // ── Selection helpers ────────────────────────────────────────────────

    /// Clear selection from a given level downward.
    pub fn clear_selection_from_tenant(&mut self) {
        self.selected_tenant = None;
        self.selected_database = None;
        self.selected_collection = None;
        self.selected_document = None;
        self.databases_status = LoadStatus::Idle;
        self.collections_status = LoadStatus::Idle;
        self.documents_status = LoadStatus::Idle;
    }

    pub fn clear_selection_from_database(&mut self) {
        self.selected_database = None;
        self.selected_collection = None;
        self.selected_document = None;
        self.collections_status = LoadStatus::Idle;
        self.documents_status = LoadStatus::Idle;
    }

    pub fn clear_selection_from_collection(&mut self) {
        self.selected_collection = None;
        self.selected_document = None;
        self.documents_status = LoadStatus::Idle;
    }

    /// Returns the number of visible columns based on selection depth.
    #[allow(dead_code)]
    pub fn visible_column_count(&self) -> usize {
        let mut count = 1; // Servers always visible
        if self.selected_server.is_some() {
            count += 1; // Tenants
            if self.selected_tenant.is_some() {
                count += 1; // Databases
                if self.selected_database.is_some() {
                    count += 1; // Collections
                    if self.selected_collection.is_some() {
                        count += 1; // Documents
                    }
                }
            }
        }
        count
    }
}

// ── Browser messages ─────────────────────────────────────────────────────────

/// Messages specific to the browser page.
#[derive(Debug, Clone)]
pub enum BrowserMsg {
    /// Column view resize event
    ColumnView(ColumnViewMessage),

    // Selection
    SelectServer(usize),
    SelectTenant(String),
    SelectDatabase(String),
    SelectCollection(String), // collection ID
    SelectDocument(String),   // document ID

    // Async data results
    TenantsLoaded {
        server_index: usize,
        result: Result<Vec<String>, String>,
    },
    DatabasesLoaded {
        server_index: usize,
        tenant: String,
        result: Result<Vec<String>, String>,
    },
    CollectionsLoaded {
        server_index: usize,
        tenant: String,
        database: String,
        result: Result<Vec<Collection>, String>,
    },
    DocumentsLoaded {
        server_index: usize,
        tenant: String,
        database: String,
        collection_id: String,
        result: Result<Vec<Document>, String>,
    },

    // Dialog
    DialogInputChanged(String),
    DialogConfirm,
    DialogCancel,

    // CRUD results
    #[allow(dead_code)]
    ServerCreated,
    TenantCreated {
        server_index: usize,
        tenant: String,
        result: Result<(), String>,
    },
    DatabaseCreated {
        server_index: usize,
        tenant: String,
        database: String,
        result: Result<(), String>,
    },
    CollectionCreated {
        server_index: usize,
        tenant: String,
        database: String,
        result: Result<Collection, String>,
    },

    // Add Server form
    StartAddServer,
    CancelAddServer,
    AddServerNameChanged(String),
    AddServerUrlChanged(String),
    AddServerAuthTokenChanged(String),
    AddServerAuthHeaderTypeChanged(String),
    TestNewServerConnection,
    TestNewServerResult(Result<(), String>),
    SaveNewServer,

    // Server edit/delete
    StartEditServer {
        index: usize,
    },
    CancelEditServer,
    SaveEditServer,
    RequestDeleteServer {
        index: usize,
    },

    // Delete actions
    RequestDeleteDatabase {
        server_index: usize,
        tenant: String,
        name: String,
    },
    RequestDeleteCollection {
        server_index: usize,
        tenant: String,
        database: String,
        collection_id: String,
        #[allow(dead_code)]
        collection_name: String,
    },
    RequestDeleteDocument {
        server_index: usize,
        tenant: String,
        database: String,
        collection_id: String,
        document_id: String,
    },
    DatabaseDeleted {
        server_index: usize,
        tenant: String,
        result: Result<(), String>,
    },
    CollectionDeleted {
        server_index: usize,
        tenant: String,
        database: String,
        result: Result<(), String>,
    },
    DocumentDeleted {
        server_index: usize,
        tenant: String,
        database: String,
        collection_id: String,
        result: Result<(), String>,
    },

    // Add-new actions (open dialog for adding)
    OpenAddTenantDialog {
        server_index: usize,
    },
    OpenAddDatabaseDialog {
        server_index: usize,
        tenant: String,
    },
    OpenAddCollectionDialog {
        server_index: usize,
        tenant: String,
        database: String,
    },

    // Menu
    ToggleMenu(MenuItemId),
    CloseMenu,

    // Filter
    ToggleServerFilter,
    FilterServers(String),
    ClearServerFilter,
}

// ── View functions ───────────────────────────────────────────────────────────

/// Main browser view — builds the column_view with one column per visible level.
pub fn view<'a, Message: Clone + 'static>(
    state: &'a BrowserState,
    servers: &'a [ServerConfig],
    on_message: impl Fn(BrowserMsg) -> Message + Copy + 'a,
    space_s: u16,
    _space_m: u16,
    _window_width: f32,
    window_height: f32,
) -> Element<'a, Message> {
    let column_height = Length::Fixed((window_height - 100.0).max(400.0));

    // Build the column_view
    let mut cv = column_view(move |msg| on_message(BrowserMsg::ColumnView(msg)))
        .id(state.scrollable_id.clone())
        .column_width(SERVER_COLUMN_WIDTH)
        .min_column_width(150.0)
        .max_column_width(600.0)
        .resizable(true)
        .spacing(space_s as f32)
        .height(column_height);

    // Column 0: Servers (always visible)
    let servers_col = build_servers_column(state, servers, on_message, space_s);
    cv = push_column(cv, servers_col, 0, state, SERVER_COLUMN_WIDTH);

    // Column 1: Tenants (visible when a server is selected)
    if let Some(server_index) = state.selected_server {
        let tenants_col = build_tenants_column(state, server_index, on_message, space_s);
        cv = push_column(cv, tenants_col, 1, state, TENANT_COLUMN_WIDTH);

        // Column 2: Databases
        if let Some(ref tenant) = state.selected_tenant {
            let dbs_col = build_databases_column(state, server_index, tenant, on_message, space_s);
            cv = push_column(cv, dbs_col, 2, state, DATABASE_COLUMN_WIDTH);

            // Column 3: Collections
            if let Some(ref database) = state.selected_database {
                let colls_col = build_collections_column(
                    state,
                    server_index,
                    tenant,
                    database,
                    on_message,
                    space_s,
                );
                cv = push_column(cv, colls_col, 3, state, COLLECTION_COLUMN_WIDTH);

                // Column 4: Documents
                if let Some(ref collection_id) = state.selected_collection {
                    let docs_col = build_documents_column(
                        state,
                        server_index,
                        tenant,
                        database,
                        collection_id,
                        on_message,
                        space_s,
                    );
                    cv = push_column(cv, docs_col, 4, state, DOCUMENT_COLUMN_WIDTH);
                }
            }
        }
    }

    let content: Element<'a, Message> = cv.into();

    // Wrap in dialog if one is open
    if let Some(ref dialog) = state.dialog {
        render_dialog(content, dialog, on_message, space_s)
    } else {
        content
    }
}

/// Push a column element with its persisted width (if any) or default.
fn push_column<'a, Message: Clone + 'a>(
    cv: cosmetics::widgets::column_view::ColumnView<'a, Message>,
    element: Element<'a, Message>,
    index: usize,
    state: &BrowserState,
    default_width: f32,
) -> cosmetics::widgets::column_view::ColumnView<'a, Message> {
    match state.column_widths.get(index).copied().flatten() {
        Some(w) => cv.push_with_width(element, w),
        None => cv.push_with_width(element, default_width),
    }
}

// ── Column builders ──────────────────────────────────────────────────────────

/// Builds the servers column.
fn build_servers_column<'a, Message: Clone + 'static>(
    state: &'a BrowserState,
    servers: &'a [ServerConfig],
    on_message: impl Fn(BrowserMsg) -> Message + Copy + 'a,
    space_s: u16,
) -> Element<'a, Message> {
    let spacing = cosmic::theme::spacing();

    // Header
    let header = if state.server_filter_visible {
        let input = widget::text_input("Filter servers...", &state.server_filter)
            .on_input(move |s| on_message(BrowserMsg::FilterServers(s)))
            .width(Length::Fill)
            .size(14);
        let clear_btn = widget::button::icon(icon::from_name("edit-clear-symbolic"))
            .extra_small()
            .on_press(on_message(BrowserMsg::ClearServerFilter));
        widget::row::with_capacity(2)
            .push(input)
            .push(clear_btn)
            .spacing(4)
            .align_y(Alignment::Center)
    } else {
        let title = widget::text::heading("Servers").width(Length::Fill);
        let filter_btn = widget::button::icon(icon::from_name("system-search-symbolic"))
            .extra_small()
            .on_press(on_message(BrowserMsg::ToggleServerFilter));
        widget::row::with_capacity(2)
            .push(title)
            .push(filter_btn)
            .spacing(4)
            .align_y(Alignment::Center)
    };

    let header_container = widget::container(header)
        .padding([spacing.space_xxs, spacing.space_xs])
        .width(Length::Fill);

    // Server items
    let filter_lower = state.server_filter.to_lowercase();
    let mut list = widget::column::with_capacity(servers.len() + 2).spacing(2);

    for (index, config) in servers.iter().enumerate() {
        // Filter
        if !filter_lower.is_empty() && !config.name.to_lowercase().contains(&filter_lower) {
            continue;
        }

        // If editing this server, show inline form
        if let Some((edit_idx, ref form)) = state.editing_server {
            if edit_idx == index {
                list = list.push(render_inline_server_form(
                    form,
                    Some(index),
                    on_message,
                    space_s,
                ));
                continue;
            }
        }

        let is_selected = state.selected_server == Some(index);
        let select_msg = on_message(BrowserMsg::SelectServer(index));

        let card = render_server_card(config, index, is_selected, &state.open_menu, on_message);
        let card = widget::mouse_area(card)
            .on_press(select_msg)
            .on_right_press(on_message(BrowserMsg::ToggleMenu(MenuItemId::Server {
                index,
            })));

        list = list.push(card);
    }

    // Add server button or inline form
    if let Some(ref form) = state.adding_server {
        list = list.push(render_inline_server_form(form, None, on_message, space_s));
    } else {
        let add_btn = widget::button::text("+ Add Server")
            .class(cosmic::theme::Button::Standard)
            .width(Length::Fill)
            .on_press(on_message(BrowserMsg::StartAddServer));
        list = list.push(add_btn);
    }

    let scrollable = widget::scrollable(list)
        .width(Length::Fill)
        .height(Length::Fill);

    widget::column::with_capacity(2)
        .push(header_container)
        .push(scrollable)
        .spacing(2)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Builds the tenants column.
fn build_tenants_column<'a, Message: Clone + 'static>(
    state: &'a BrowserState,
    server_index: usize,
    on_message: impl Fn(BrowserMsg) -> Message + Copy + 'a,
    _space_s: u16,
) -> Element<'a, Message> {
    let spacing = cosmic::theme::spacing();

    // Header with add button
    let title = widget::text::heading("Tenants").width(Length::Fill);
    let add_btn = widget::button::icon(icon::from_name("list-add-symbolic"))
        .extra_small()
        .on_press(on_message(BrowserMsg::OpenAddTenantDialog { server_index }));
    let header = widget::container(
        widget::row::with_capacity(2)
            .push(title)
            .push(add_btn)
            .spacing(4)
            .align_y(Alignment::Center),
    )
    .padding([spacing.space_xxs, spacing.space_xs])
    .width(Length::Fill);

    // Content based on load status
    let content: Element<'a, Message> = match &state.tenants_status {
        LoadStatus::Idle => widget::text::body("Select a server").into(),
        LoadStatus::Loading => widget::container(widget::text::body("Loading tenants..."))
            .width(Length::Fill)
            .center(Length::Fill)
            .into(),
        LoadStatus::Error(e) => widget::container(
            widget::column::with_capacity(2)
                .push(icon::from_name("dialog-error-symbolic").size(24))
                .push(widget::text::caption(e.clone()).class(cosmic::style::Text::Accent))
                .spacing(8)
                .align_x(Alignment::Center),
        )
        .width(Length::Fill)
        .center(Length::Fill)
        .into(),
        LoadStatus::Loaded => {
            let tenants = state.tenants_cache.get(&server_index);
            match tenants {
                Some(tenants) if !tenants.is_empty() => {
                    let mut list = widget::column::with_capacity(tenants.len()).spacing(2);
                    for name in tenants {
                        let is_selected = state.selected_tenant.as_deref() == Some(name.as_str());
                        let select_msg = on_message(BrowserMsg::SelectTenant(name.clone()));
                        let card = render_tenant_card(name, is_selected);
                        let card = widget::mouse_area(card).on_press(select_msg);
                        list = list.push(card);
                    }
                    widget::scrollable(list)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .into()
                }
                _ => widget::text::body("No tenants found").into(),
            }
        }
    };

    widget::column::with_capacity(2)
        .push(header)
        .push(content)
        .spacing(2)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Builds the databases column.
fn build_databases_column<'a, Message: Clone + 'static>(
    state: &'a BrowserState,
    server_index: usize,
    tenant: &str,
    on_message: impl Fn(BrowserMsg) -> Message + Copy + 'a,
    _space_s: u16,
) -> Element<'a, Message> {
    let spacing = cosmic::theme::spacing();
    let tenant_owned = tenant.to_string();

    // Header with add button
    let title = widget::text::heading("Databases").width(Length::Fill);
    let add_btn = widget::button::icon(icon::from_name("list-add-symbolic"))
        .extra_small()
        .on_press(on_message(BrowserMsg::OpenAddDatabaseDialog {
            server_index,
            tenant: tenant_owned.clone(),
        }));
    let header = widget::container(
        widget::row::with_capacity(2)
            .push(title)
            .push(add_btn)
            .spacing(4)
            .align_y(Alignment::Center),
    )
    .padding([spacing.space_xxs, spacing.space_xs])
    .width(Length::Fill);

    let cache_key = format!("{}:{}", server_index, tenant);

    let content: Element<'a, Message> = match &state.databases_status {
        LoadStatus::Idle => widget::text::body("Select a tenant").into(),
        LoadStatus::Loading => widget::container(widget::text::body("Loading databases..."))
            .width(Length::Fill)
            .center(Length::Fill)
            .into(),
        LoadStatus::Error(e) => widget::container(
            widget::column::with_capacity(2)
                .push(icon::from_name("dialog-error-symbolic").size(24))
                .push(widget::text::caption(e.clone()).class(cosmic::style::Text::Accent))
                .spacing(8)
                .align_x(Alignment::Center),
        )
        .width(Length::Fill)
        .center(Length::Fill)
        .into(),
        LoadStatus::Loaded => {
            let databases = state.databases_cache.get(&cache_key);
            match databases {
                Some(databases) if !databases.is_empty() => {
                    let mut list = widget::column::with_capacity(databases.len()).spacing(2);
                    for name in databases {
                        let is_selected = state.selected_database.as_deref() == Some(name.as_str());
                        let select_msg = on_message(BrowserMsg::SelectDatabase(name.clone()));
                        let card = render_database_card(
                            name,
                            is_selected,
                            server_index,
                            tenant_owned.clone(),
                            &state.open_menu,
                            on_message,
                        );
                        let tenant_for_menu = tenant_owned.clone();
                        let name_for_menu = name.clone();
                        let card = widget::mouse_area(card)
                            .on_press(select_msg)
                            .on_right_press(on_message(BrowserMsg::ToggleMenu(
                                MenuItemId::Database {
                                    server_index,
                                    tenant: tenant_for_menu,
                                    name: name_for_menu,
                                },
                            )));
                        list = list.push(card);
                    }
                    widget::scrollable(list)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .into()
                }
                _ => widget::text::body("No databases found").into(),
            }
        }
    };

    widget::column::with_capacity(2)
        .push(header)
        .push(content)
        .spacing(2)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Builds the collections column.
fn build_collections_column<'a, Message: Clone + 'static>(
    state: &'a BrowserState,
    server_index: usize,
    tenant: &str,
    database: &str,
    on_message: impl Fn(BrowserMsg) -> Message + Copy + 'a,
    _space_s: u16,
) -> Element<'a, Message> {
    let spacing = cosmic::theme::spacing();
    let tenant_owned = tenant.to_string();
    let database_owned = database.to_string();

    // Header with add button
    let title = widget::text::heading("Collections").width(Length::Fill);
    let add_btn = widget::button::icon(icon::from_name("list-add-symbolic"))
        .extra_small()
        .on_press(on_message(BrowserMsg::OpenAddCollectionDialog {
            server_index,
            tenant: tenant_owned.clone(),
            database: database_owned.clone(),
        }));
    let header = widget::container(
        widget::row::with_capacity(2)
            .push(title)
            .push(add_btn)
            .spacing(4)
            .align_y(Alignment::Center),
    )
    .padding([spacing.space_xxs, spacing.space_xs])
    .width(Length::Fill);

    let cache_key = format!("{}:{}:{}", server_index, tenant, database);

    let content: Element<'a, Message> = match &state.collections_status {
        LoadStatus::Idle => widget::text::body("Select a database").into(),
        LoadStatus::Loading => widget::container(widget::text::body("Loading collections..."))
            .width(Length::Fill)
            .center(Length::Fill)
            .into(),
        LoadStatus::Error(e) => widget::container(
            widget::column::with_capacity(2)
                .push(icon::from_name("dialog-error-symbolic").size(24))
                .push(widget::text::caption(e.clone()).class(cosmic::style::Text::Accent))
                .spacing(8)
                .align_x(Alignment::Center),
        )
        .width(Length::Fill)
        .center(Length::Fill)
        .into(),
        LoadStatus::Loaded => {
            let collections = state.collections_cache.get(&cache_key);
            match collections {
                Some(collections) if !collections.is_empty() => {
                    let mut list = widget::column::with_capacity(collections.len()).spacing(2);
                    for collection in collections {
                        let is_selected =
                            state.selected_collection.as_deref() == Some(collection.id.as_str());
                        let select_msg =
                            on_message(BrowserMsg::SelectCollection(collection.id.clone()));
                        let card = render_collection_card(
                            collection,
                            is_selected,
                            server_index,
                            tenant_owned.clone(),
                            database_owned.clone(),
                            &state.open_menu,
                            on_message,
                        );
                        let tenant_for_menu = tenant_owned.clone();
                        let db_for_menu = database_owned.clone();
                        let coll_id = collection.id.clone();
                        let card = widget::mouse_area(card)
                            .on_press(select_msg)
                            .on_right_press(on_message(BrowserMsg::ToggleMenu(
                                MenuItemId::Collection {
                                    server_index,
                                    tenant: tenant_for_menu,
                                    database: db_for_menu,
                                    collection_id: coll_id,
                                },
                            )));
                        list = list.push(card);
                    }
                    widget::scrollable(list)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .into()
                }
                _ => widget::text::body("No collections found").into(),
            }
        }
    };

    widget::column::with_capacity(2)
        .push(header)
        .push(content)
        .spacing(2)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Builds the documents column.
fn build_documents_column<'a, Message: Clone + 'static>(
    state: &'a BrowserState,
    server_index: usize,
    tenant: &str,
    database: &str,
    collection_id: &str,
    on_message: impl Fn(BrowserMsg) -> Message + Copy + 'a,
    _space_s: u16,
) -> Element<'a, Message> {
    let spacing = cosmic::theme::spacing();
    let tenant_owned = tenant.to_string();
    let database_owned = database.to_string();
    let collection_id_owned = collection_id.to_string();

    // Header
    let title = widget::text::heading("Documents").width(Length::Fill);
    let header = widget::container(
        widget::row::with_capacity(1)
            .push(title)
            .align_y(Alignment::Center),
    )
    .padding([spacing.space_xxs, spacing.space_xs])
    .width(Length::Fill);

    let cache_key = format!("{}:{}:{}:{}", server_index, tenant, database, collection_id);

    let content: Element<'a, Message> = match &state.documents_status {
        LoadStatus::Idle => widget::text::body("Select a collection").into(),
        LoadStatus::Loading => widget::container(widget::text::body("Loading documents..."))
            .width(Length::Fill)
            .center(Length::Fill)
            .into(),
        LoadStatus::Error(e) => widget::container(
            widget::column::with_capacity(2)
                .push(icon::from_name("dialog-error-symbolic").size(24))
                .push(widget::text::caption(e.clone()).class(cosmic::style::Text::Accent))
                .spacing(8)
                .align_x(Alignment::Center),
        )
        .width(Length::Fill)
        .center(Length::Fill)
        .into(),
        LoadStatus::Loaded => {
            let documents = state.documents_cache.get(&cache_key);
            match documents {
                Some(documents) if !documents.is_empty() => {
                    let mut list = widget::column::with_capacity(documents.len()).spacing(2);
                    for doc in documents {
                        let is_selected = state
                            .selected_document
                            .as_ref()
                            .map(|d| d.id == doc.id)
                            .unwrap_or(false);
                        let select_msg = on_message(BrowserMsg::SelectDocument(doc.id.clone()));
                        let card = render_document_card(
                            doc,
                            is_selected,
                            server_index,
                            tenant_owned.clone(),
                            database_owned.clone(),
                            collection_id_owned.clone(),
                            &state.open_menu,
                            on_message,
                        );
                        let tenant_for_menu = tenant_owned.clone();
                        let db_for_menu = database_owned.clone();
                        let coll_for_menu = collection_id_owned.clone();
                        let doc_id = doc.id.clone();
                        let card = widget::mouse_area(card)
                            .on_press(select_msg)
                            .on_right_press(on_message(BrowserMsg::ToggleMenu(
                                MenuItemId::Document {
                                    server_index,
                                    tenant: tenant_for_menu,
                                    database: db_for_menu,
                                    collection_id: coll_for_menu,
                                    document_id: doc_id,
                                },
                            )));
                        list = list.push(card);
                    }
                    widget::scrollable(list)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .into()
                }
                Some(_) => widget::text::body("No documents found").into(),
                None => widget::text::body("No documents found").into(),
            }
        }
    };

    widget::column::with_capacity(2)
        .push(header)
        .push(content)
        .spacing(2)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

// ── Card renderers ───────────────────────────────────────────────────────────

/// Renders a server card.
fn render_server_card<'a, Message: Clone + 'static>(
    config: &ServerConfig,
    index: usize,
    is_selected: bool,
    open_menu: &Option<MenuItemId>,
    on_message: impl Fn(BrowserMsg) -> Message + Copy + 'a,
) -> Element<'a, Message> {
    let name = config.name.clone();
    let url = config.server_url.clone();

    let menu_id = MenuItemId::Server { index };
    let is_menu_open = open_menu.as_ref() == Some(&menu_id);

    // Context menu content
    let menu_content: Element<'a, Message> = widget::container(
        widget::column::with_capacity(2)
            .push(
                widget::button::text("Edit")
                    .on_press(on_message(BrowserMsg::StartEditServer { index }))
                    .width(Length::Fill),
            )
            .push(
                widget::button::destructive("Delete")
                    .on_press(on_message(BrowserMsg::RequestDeleteServer { index }))
                    .width(Length::Fill),
            )
            .spacing(4),
    )
    .padding(4)
    .width(Length::Fixed(120.0))
    .class(cosmic::style::Container::Card)
    .into();

    let card_content = widget::row::with_capacity(3)
        .push(icon::from_name("network-server-symbolic").size(16))
        .push(
            widget::column::with_capacity(2)
                .push(widget::text::body(name).class(cosmic::style::Text::Default))
                .push(widget::text::caption(url).class(cosmic::style::Text::Accent))
                .spacing(2)
                .width(Length::Fill),
        )
        .push(icon::from_name("go-next-symbolic").size(12))
        .align_y(Alignment::Center)
        .spacing(8);

    let container_class = if is_selected {
        cosmic::style::Container::Primary
    } else {
        cosmic::style::Container::default()
    };

    let card: Element<'a, Message> = widget::container(card_content)
        .padding(10)
        .width(Length::Fill)
        .class(container_class)
        .into();

    if is_menu_open {
        popover(card)
            .popup(menu_content)
            .position(popover::Position::Bottom)
            .on_close(on_message(BrowserMsg::CloseMenu))
            .into()
    } else {
        card
    }
}

/// Renders a tenant card.
fn render_tenant_card<'a, Message: 'static>(name: &str, is_selected: bool) -> Element<'a, Message> {
    let row = widget::row::with_capacity(3)
        .push(icon::from_name("system-users-symbolic").size(16))
        .push(
            widget::text::body(name.to_string())
                .width(Length::Fill)
                .class(cosmic::style::Text::Default),
        )
        .push(icon::from_name("go-next-symbolic").size(12))
        .align_y(Alignment::Center)
        .spacing(8);

    let container_class = if is_selected {
        cosmic::style::Container::Primary
    } else {
        cosmic::style::Container::default()
    };

    widget::container(row)
        .padding([6, 10])
        .width(Length::Fill)
        .class(container_class)
        .into()
}

/// Renders a database card with context menu.
fn render_database_card<'a, Message: Clone + 'static>(
    name: &str,
    is_selected: bool,
    server_index: usize,
    tenant: String,
    open_menu: &Option<MenuItemId>,
    on_message: impl Fn(BrowserMsg) -> Message + Copy + 'a,
) -> Element<'a, Message> {
    let db_name = name.to_string();
    let menu_id = MenuItemId::Database {
        server_index,
        tenant: tenant.clone(),
        name: db_name.clone(),
    };
    let is_menu_open = open_menu.as_ref() == Some(&menu_id);

    let menu_content: Element<'a, Message> = widget::container(
        widget::button::destructive("Delete")
            .on_press(on_message(BrowserMsg::RequestDeleteDatabase {
                server_index,
                tenant,
                name: db_name.clone(),
            }))
            .width(Length::Fill),
    )
    .padding(4)
    .width(Length::Fixed(120.0))
    .class(cosmic::style::Container::Card)
    .into();

    let row = widget::row::with_capacity(3)
        .push(icon::from_name("drive-harddisk-symbolic").size(16))
        .push(
            widget::text::body(db_name)
                .width(Length::Fill)
                .class(cosmic::style::Text::Default),
        )
        .push(icon::from_name("go-next-symbolic").size(12))
        .align_y(Alignment::Center)
        .spacing(8);

    let container_class = if is_selected {
        cosmic::style::Container::Primary
    } else {
        cosmic::style::Container::default()
    };

    let card: Element<'a, Message> = widget::container(row)
        .padding([6, 10])
        .width(Length::Fill)
        .class(container_class)
        .into();

    if is_menu_open {
        popover(card)
            .popup(menu_content)
            .position(popover::Position::Bottom)
            .on_close(on_message(BrowserMsg::CloseMenu))
            .into()
    } else {
        card
    }
}

/// Renders a collection card with context menu.
fn render_collection_card<'a, Message: Clone + 'static>(
    collection: &Collection,
    is_selected: bool,
    server_index: usize,
    tenant: String,
    database: String,
    open_menu: &Option<MenuItemId>,
    on_message: impl Fn(BrowserMsg) -> Message + Copy + 'a,
) -> Element<'a, Message> {
    let collection_id = collection.id.clone();
    let collection_name = collection.name.clone();

    let menu_id = MenuItemId::Collection {
        server_index,
        tenant: tenant.clone(),
        database: database.clone(),
        collection_id: collection_id.clone(),
    };
    let is_menu_open = open_menu.as_ref() == Some(&menu_id);

    let menu_content: Element<'a, Message> = widget::container(
        widget::button::destructive("Delete")
            .on_press(on_message(BrowserMsg::RequestDeleteCollection {
                server_index,
                tenant,
                database,
                collection_id,
                collection_name: collection_name.clone(),
            }))
            .width(Length::Fill),
    )
    .padding(4)
    .width(Length::Fixed(120.0))
    .class(cosmic::style::Container::Card)
    .into();

    let metadata_count = collection.metadata.as_ref().map(|m| m.len()).unwrap_or(0);

    let header_row = widget::row::with_capacity(3)
        .push(icon::from_name("folder-symbolic").size(16))
        .push(
            widget::text::body(collection_name)
                .width(Length::Fill)
                .class(cosmic::style::Text::Default),
        )
        .push(icon::from_name("go-next-symbolic").size(12))
        .align_y(Alignment::Center)
        .spacing(8);

    let metadata_info = if metadata_count > 0 {
        widget::text::caption(format!("{} metadata fields", metadata_count))
    } else {
        widget::text::caption("No metadata")
    }
    .class(cosmic::style::Text::Accent);

    let card_content = widget::column::with_capacity(2)
        .push(header_row)
        .push(metadata_info)
        .spacing(4);

    let container_class = if is_selected {
        cosmic::style::Container::Primary
    } else {
        cosmic::style::Container::default()
    };

    let card: Element<'a, Message> = widget::container(card_content)
        .padding(10)
        .width(Length::Fill)
        .class(container_class)
        .into();

    if is_menu_open {
        popover(card)
            .popup(menu_content)
            .position(popover::Position::Bottom)
            .on_close(on_message(BrowserMsg::CloseMenu))
            .into()
    } else {
        card
    }
}

/// Renders a document card with context menu.
fn render_document_card<'a, Message: Clone + 'static>(
    doc: &Document,
    is_selected: bool,
    server_index: usize,
    tenant: String,
    database: String,
    collection_id: String,
    open_menu: &Option<MenuItemId>,
    on_message: impl Fn(BrowserMsg) -> Message + Copy + 'a,
) -> Element<'a, Message> {
    let doc_id = doc.id.clone();

    let menu_id = MenuItemId::Document {
        server_index,
        tenant: tenant.clone(),
        database: database.clone(),
        collection_id: collection_id.clone(),
        document_id: doc_id.clone(),
    };
    let is_menu_open = open_menu.as_ref() == Some(&menu_id);

    let menu_content: Element<'a, Message> = widget::container(
        widget::button::destructive("Delete")
            .on_press(on_message(BrowserMsg::RequestDeleteDocument {
                server_index,
                tenant,
                database,
                collection_id,
                document_id: doc_id.clone(),
            }))
            .width(Length::Fill),
    )
    .padding(4)
    .width(Length::Fixed(120.0))
    .class(cosmic::style::Container::Card)
    .into();

    // Content preview
    let content_preview = doc
        .document
        .as_ref()
        .map(|s| truncate_str(s, 80))
        .unwrap_or_else(|| "[No content]".to_string());

    let metadata_count = doc.metadata.as_ref().map(|m| m.len()).unwrap_or(0);

    let header_row = widget::row::with_capacity(2)
        .push(icon::from_name("text-x-generic-symbolic").size(16))
        .push(
            widget::text::body(doc_id)
                .width(Length::Fill)
                .class(cosmic::style::Text::Default),
        )
        .align_y(Alignment::Center)
        .spacing(8);

    let content_text = widget::text::caption(content_preview).class(cosmic::style::Text::Default);
    let metadata_info = widget::text::caption(format!("{} metadata fields", metadata_count))
        .class(cosmic::style::Text::Accent);

    let card_content = widget::column::with_capacity(3)
        .push(header_row)
        .push(content_text)
        .push(metadata_info)
        .spacing(4);

    let container_class = if is_selected {
        cosmic::style::Container::Primary
    } else {
        cosmic::style::Container::default()
    };

    let card: Element<'a, Message> = widget::container(card_content)
        .padding(10)
        .width(Length::Fill)
        .class(container_class)
        .into();

    if is_menu_open {
        popover(card)
            .popup(menu_content)
            .position(popover::Position::Bottom)
            .on_close(on_message(BrowserMsg::CloseMenu))
            .into()
    } else {
        card
    }
}

// ── Inline forms and dialogs ─────────────────────────────────────────────────

/// Renders an inline server form for add/edit within the column.
fn render_inline_server_form<'a, Message: Clone + 'static>(
    form: &'a AddServerForm,
    edit_index: Option<usize>,
    on_message: impl Fn(BrowserMsg) -> Message + Copy + 'a,
    space_s: u16,
) -> Element<'a, Message> {
    let is_editing = edit_index.is_some();
    let title = if is_editing {
        "Edit Server"
    } else {
        "Add Server"
    };

    let mut content = widget::column::with_capacity(12).spacing(4);

    content = content.push(widget::text::body(title).class(cosmic::style::Text::Accent));

    content = content.push(widget::text::caption("Name"));
    content = content.push(
        widget::text_input("My Server", &form.name)
            .on_input(move |s| on_message(BrowserMsg::AddServerNameChanged(s)))
            .width(Length::Fill)
            .padding(4),
    );

    content = content.push(widget::text::caption("URL"));
    content = content.push(
        widget::text_input("http://localhost:8000", &form.url)
            .on_input(move |s| on_message(BrowserMsg::AddServerUrlChanged(s)))
            .width(Length::Fill)
            .padding(4),
    );

    content = content.push(widget::text::caption("Auth Token"));
    content = content.push(
        widget::secure_input("Token", &form.auth_token, None, true)
            .on_input(move |s| on_message(BrowserMsg::AddServerAuthTokenChanged(s)))
            .width(Length::Fill)
            .padding(4),
    );

    content = content.push(widget::text::caption("Auth Type"));
    content = content.push(
        widget::row::with_capacity(2)
            .push(
                widget::button::text("Bearer")
                    .class(if form.auth_header_type == "authorization" {
                        cosmic::theme::Button::Suggested
                    } else {
                        cosmic::theme::Button::Standard
                    })
                    .padding([2, 8])
                    .on_press(on_message(BrowserMsg::AddServerAuthHeaderTypeChanged(
                        "authorization".to_string(),
                    ))),
            )
            .push(
                widget::button::text("X-Chroma")
                    .class(if form.auth_header_type == "x-chroma-token" {
                        cosmic::theme::Button::Suggested
                    } else {
                        cosmic::theme::Button::Standard
                    })
                    .padding([2, 8])
                    .on_press(on_message(BrowserMsg::AddServerAuthHeaderTypeChanged(
                        "x-chroma-token".to_string(),
                    ))),
            )
            .spacing(4),
    );

    // Status message
    match &form.status {
        AddServerStatus::Editing => {}
        AddServerStatus::Testing => {
            content = content.push(widget::text::caption("Testing..."));
        }
        AddServerStatus::TestSuccess => {
            content = content.push(widget::text::caption("Connected!").class(
                cosmic::style::Text::Color(cosmic::iced::Color::from_rgb(0.2, 0.8, 0.2)),
            ));
        }
        AddServerStatus::TestFailed(err) => {
            let short_err = truncate_str(err, 30);
            content = content.push(
                widget::text::caption(format!("Failed: {short_err}"))
                    .class(cosmic::style::Text::Accent),
            );
        }
    }

    let is_testing = form.status == AddServerStatus::Testing;
    let can_save = form.status == AddServerStatus::TestSuccess && !form.name.is_empty();

    let cancel_msg = if is_editing {
        on_message(BrowserMsg::CancelEditServer)
    } else {
        on_message(BrowserMsg::CancelAddServer)
    };

    let mut button_row = widget::row::with_capacity(3).spacing(4);
    button_row = button_row.push(
        widget::button::text("Cancel")
            .class(cosmic::theme::Button::Standard)
            .padding([4, 8])
            .on_press(cancel_msg),
    );

    if !is_testing {
        button_row = button_row.push(
            widget::button::text("Test")
                .class(cosmic::theme::Button::Standard)
                .padding([4, 8])
                .on_press(on_message(BrowserMsg::TestNewServerConnection)),
        );
    }

    content = content.push(button_row);

    if can_save {
        let save_msg = if is_editing {
            on_message(BrowserMsg::SaveEditServer)
        } else {
            on_message(BrowserMsg::SaveNewServer)
        };
        content = content.push(
            widget::button::suggested("Save")
                .width(Length::Fill)
                .on_press(save_msg),
        );
    }

    widget::container(content)
        .padding(space_s)
        .width(Length::Fill)
        .class(cosmic::style::Container::Card)
        .into()
}

/// Renders a dialog for adding new items or confirming actions.
fn render_dialog<'a, Message: Clone + 'static>(
    background: Element<'a, Message>,
    dialog: &'a BrowserDialog,
    on_message: impl Fn(BrowserMsg) -> Message + Copy + 'a,
    space_s: u16,
) -> Element<'a, Message> {
    let dialog_widget: Element<'a, Message> = match dialog {
        BrowserDialog::ConfirmCreateTenant { tenant, .. } => {
            let dialog_content = widget::column::with_capacity(2)
                .push(widget::text::body(format!(
                    "Tenant '{}' doesn't exist on the server. Would you like to create it?",
                    tenant
                )))
                .push(
                    widget::row::with_capacity(2)
                        .push(
                            widget::button::standard("Cancel")
                                .on_press(on_message(BrowserMsg::DialogCancel)),
                        )
                        .push(
                            widget::button::suggested("Create Tenant")
                                .on_press(on_message(BrowserMsg::DialogConfirm)),
                        )
                        .spacing(space_s),
                )
                .spacing(space_s);

            widget::dialog()
                .title("Create Tenant?")
                .control(dialog_content)
                .into()
        }
        _ => {
            let (title, placeholder) = match dialog {
                BrowserDialog::AddServer { .. } => ("Add Server", "Server name"),
                BrowserDialog::AddTenant { .. } => ("Add Tenant", "Tenant name"),
                BrowserDialog::AddDatabase { .. } => ("Add Database", "Database name"),
                BrowserDialog::AddCollection { .. } => ("Add Collection", "Collection name"),
                BrowserDialog::ConfirmCreateTenant { .. } => unreachable!(),
            };

            let value = match dialog {
                BrowserDialog::AddServer { name } => name,
                BrowserDialog::AddTenant { name, .. } => name,
                BrowserDialog::AddDatabase { name, .. } => name,
                BrowserDialog::AddCollection { name, .. } => name,
                BrowserDialog::ConfirmCreateTenant { .. } => unreachable!(),
            };

            let dialog_content = widget::column::with_capacity(2)
                .push(
                    widget::text_input(placeholder, value)
                        .on_input(move |s| on_message(BrowserMsg::DialogInputChanged(s)))
                        .on_submit(move |_| on_message(BrowserMsg::DialogConfirm))
                        .width(Length::Fixed(300.0)),
                )
                .push(
                    widget::row::with_capacity(2)
                        .push(
                            widget::button::standard("Cancel")
                                .on_press(on_message(BrowserMsg::DialogCancel)),
                        )
                        .push(
                            widget::button::suggested("Create")
                                .on_press(on_message(BrowserMsg::DialogConfirm)),
                        )
                        .spacing(space_s),
                )
                .spacing(space_s);

            widget::dialog().title(title).control(dialog_content).into()
        }
    };

    widget::popover(background)
        .modal(true)
        .popup(dialog_widget)
        .into()
}
