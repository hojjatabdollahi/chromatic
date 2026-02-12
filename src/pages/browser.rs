// SPDX-License-Identifier: MPL-2.0

//! Browser page using `column_view` for hierarchical navigation.
//!
//! Column hierarchy:
//! 1. Server configs (+ Add New inline form)
//! 2. Tenants (+ Add New inline form)
//! 3. Databases (+ Add New)
//! 4. Collections (+ Add New)
//! 5. Documents
//!
//! Each column is a fully custom `Element` built on the app side, with
//! the `column_view` widget from cosmetics handling horizontal layout,
//! scrolling, and resize handles.

use crate::api::{Collection, Document};
use crate::config::ServerConfig;
use cosmetics::widgets::column_view::{column_view, ColumnViewMessage};
use cosmic::iced::{Alignment, Length};
use cosmic::iced_widget::scrollable::{snap_to, RelativeOffset};
use cosmic::prelude::*;
use cosmic::widget::{self, context_menu, icon, menu, Id};
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

/// Spacing between columns (in pixels).
const COLUMN_SPACING: f32 = 2.0;

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

/// Form state for adding/editing a tenant (just a name).
#[derive(Debug, Clone, Default)]
pub struct AddTenantForm {
    pub name: String,
    /// `Some(old_name)` when editing, `None` when adding.
    pub editing: Option<String>,
}

/// Dialog types for adding new items.
#[derive(Debug, Clone)]
pub enum BrowserDialog {
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
    ConfirmCreateTenant { server_index: usize, tenant: String },
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

    /// Dialog state
    pub dialog: Option<BrowserDialog>,
    /// Add server form (shown inline in the server column)
    pub adding_server: Option<AddServerForm>,
    /// Edit server form (server_index, form state)
    pub editing_server: Option<(usize, AddServerForm)>,
    /// Add/edit tenant form (shown inline in the tenant column)
    pub adding_tenant: Option<AddTenantForm>,

    /// Scrollable ID for the `column_view`
    pub scrollable_id: Id,

    /// Context for the currently right-clicked item (for menu actions that need it)
    pub menu_context: Option<MenuContext>,

    // Filters per column
    pub server_filter: String,
    pub server_filter_visible: bool,
    pub tenant_filter: String,
    pub tenant_filter_visible: bool,
    pub database_filter: String,
    pub database_filter_visible: bool,
    pub collection_filter: String,
    pub collection_filter_visible: bool,
    pub document_filter: String,
    pub document_filter_visible: bool,
}

/// Context saved when a right-click menu is shown, so menu actions know what to act on.
#[derive(Debug, Clone)]
pub enum MenuContext {
    Tenant {
        server_index: usize,
        name: String,
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
        collection_name: String,
    },
    Document {
        server_index: usize,
        tenant: String,
        database: String,
        collection_id: String,
        document_id: String,
    },
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
            adding_tenant: None,
            scrollable_id: Id::unique(),
            menu_context: None,
            server_filter: String::new(),
            server_filter_visible: false,
            tenant_filter: String::new(),
            tenant_filter_visible: false,
            database_filter: String::new(),
            database_filter_visible: false,
            collection_filter: String::new(),
            collection_filter_visible: false,
            document_filter: String::new(),
            document_filter_visible: false,
        }
    }
}

impl BrowserState {
    /// Creates new browser state from server configs.
    pub fn new(servers: &[ServerConfig]) -> Self {
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

    /// Returns a task that scrolls the `column_view` to the right.
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
        let key = format!("{server_index}:{tenant}");
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
        let key = format!("{server_index}:{tenant}:{database}");
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
        let key = format!("{server_index}:{tenant}:{database}:{collection_id}");
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
    SelectCollection(String),
    SelectDocument(String),

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

    // Add/edit tenant form
    StartAddTenant,
    StartEditTenant {
        name: String,
    },
    CancelTenantForm,
    TenantFormNameChanged(String),
    SaveTenant,

    // Menu-dispatched actions (resolved via `menu_context`)
    EditTenantFromMenu,
    DeleteDatabaseFromMenu,
    DeleteCollectionFromMenu,
    DeleteDocumentFromMenu,

    // Delete actions (resolved)
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

    // Add-new actions (open dialog)
    OpenAddDatabaseDialog {
        server_index: usize,
        tenant: String,
    },
    OpenAddCollectionDialog {
        server_index: usize,
        tenant: String,
        database: String,
    },

    // Save menu context when right-clicking
    SetMenuContext(MenuContext),

    // Filter
    ToggleFilter(ColumnLevel),
    UpdateFilter(ColumnLevel, String),
    ClearFilter(ColumnLevel),
}

/// Identifies a column level for filter messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnLevel {
    Servers,
    Tenants,
    Databases,
    Collections,
    Documents,
}

// ── View functions ───────────────────────────────────────────────────────────

/// Main browser view.
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

    let mut cv = column_view(move |msg| on_message(BrowserMsg::ColumnView(msg)))
        .id(state.scrollable_id.clone())
        .column_width(SERVER_COLUMN_WIDTH)
        .min_column_width(150.0)
        .max_column_width(600.0)
        .resizable(true)
        .spacing(COLUMN_SPACING)
        .height(column_height);

    // Column 0: Servers (always visible)
    let servers_col = build_servers_column(state, servers, on_message, space_s);
    cv = push_column(cv, servers_col, 0, state, SERVER_COLUMN_WIDTH);

    // Column 1: Tenants
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

    if let Some(ref dialog) = state.dialog {
        render_dialog(content, dialog, on_message, space_s)
    } else {
        content
    }
}

/// Push a column element with its persisted or default width.
fn push_column<'a, Message: Clone + 'a>(
    cv: cosmetics::widgets::column_view::ColumnView<'a, Message>,
    element: Element<'a, Message>,
    index: usize,
    state: &BrowserState,
    default_width: f32,
) -> cosmetics::widgets::column_view::ColumnView<'a, Message> {
    let w = state
        .column_widths
        .get(index)
        .copied()
        .flatten()
        .unwrap_or(default_width);
    cv.push_with_width(element, w)
}

// ── Shared column header builder ─────────────────────────────────────────────

/// Builds a standard column header with title, optional filter, optional add button, and divider.
fn build_column_header<'a, Message: Clone + 'static>(
    title: &str,
    level: ColumnLevel,
    filter_text: &'a str,
    filter_visible: bool,
    show_add_button: bool,
    on_message: impl Fn(BrowserMsg) -> Message + Copy + 'a,
) -> Element<'a, Message> {
    let spacing = cosmic::theme::spacing();

    let header_row = if filter_visible {
        let input = widget::text_input("Filter...", filter_text)
            .on_input(move |s| on_message(BrowserMsg::UpdateFilter(level, s)))
            .width(Length::Fill)
            .size(14);
        let clear_btn = widget::button::icon(icon::from_name("edit-clear-symbolic"))
            .extra_small()
            .on_press(on_message(BrowserMsg::ClearFilter(level)));
        widget::row::with_capacity(2)
            .push(input)
            .push(clear_btn)
            .spacing(4)
            .align_y(Alignment::Center)
    } else {
        let title_widget = widget::text::heading(title.to_string()).width(Length::Fill);
        let mut row = widget::row::with_capacity(3)
            .push(title_widget)
            .spacing(4)
            .align_y(Alignment::Center);

        let search_btn = widget::button::icon(icon::from_name("system-search-symbolic"))
            .extra_small()
            .on_press(on_message(BrowserMsg::ToggleFilter(level)));
        row = row.push(search_btn);

        if show_add_button {
            let add_btn = widget::button::icon(icon::from_name("list-add-symbolic"))
                .extra_small()
                .on_press(match level {
                    ColumnLevel::Servers => on_message(BrowserMsg::StartAddServer),
                    ColumnLevel::Tenants => on_message(BrowserMsg::StartAddTenant),
                    // Databases/Collections use dialogs, handled by caller
                    _ => on_message(BrowserMsg::ToggleFilter(level)), // placeholder, overridden
                });
            row = row.push(add_btn);
        }
        row
    };

    let header_container = widget::container(header_row)
        .padding([spacing.space_xxs, spacing.space_xs])
        .width(Length::Fill);

    let divider = widget::divider::horizontal::default();

    widget::column::with_capacity(2)
        .push(header_container)
        .push(divider)
        .width(Length::Fill)
        .into()
}

// ── Column builders ──────────────────────────────────────────────────────────

/// Builds the servers column.
fn build_servers_column<'a, Message: Clone + 'static>(
    state: &'a BrowserState,
    servers: &'a [ServerConfig],
    on_message: impl Fn(BrowserMsg) -> Message + Copy + 'a,
    space_s: u16,
) -> Element<'a, Message> {
    let header = build_column_header(
        "Servers",
        ColumnLevel::Servers,
        &state.server_filter,
        state.server_filter_visible,
        true,
        on_message,
    );

    let filter_lower = state.server_filter.to_lowercase();
    let mut list = widget::column::with_capacity(servers.len() + 2).spacing(2);

    for (index, config) in servers.iter().enumerate() {
        if !filter_lower.is_empty() && !config.name.to_lowercase().contains(&filter_lower) {
            continue;
        }

        // Inline edit form
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
        let card = render_server_card(config, index, is_selected, on_message);
        list = list.push(card);
    }

    // Inline add form at bottom
    if let Some(ref form) = state.adding_server {
        list = list.push(render_inline_server_form(form, None, on_message, space_s));
    }

    let scrollable = widget::scrollable(list)
        .width(Length::Fill)
        .height(Length::Fill);

    widget::column::with_capacity(2)
        .push(header)
        .push(scrollable)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Builds the tenants column.
fn build_tenants_column<'a, Message: Clone + 'static>(
    state: &'a BrowserState,
    server_index: usize,
    on_message: impl Fn(BrowserMsg) -> Message + Copy + 'a,
    space_s: u16,
) -> Element<'a, Message> {
    let header = build_column_header(
        "Tenants",
        ColumnLevel::Tenants,
        &state.tenant_filter,
        state.tenant_filter_visible,
        true,
        on_message,
    );

    let filter_lower = state.tenant_filter.to_lowercase();

    let content: Element<'a, Message> = match &state.tenants_status {
        LoadStatus::Idle => widget::text::body("Select a server").into(),
        LoadStatus::Loading => render_loading("Loading tenants..."),
        LoadStatus::Error(e) => render_error(e),
        LoadStatus::Loaded => {
            let tenants = state.tenants_cache.get(&server_index);
            match tenants {
                Some(tenants) if !tenants.is_empty() => {
                    let mut list = widget::column::with_capacity(tenants.len() + 1).spacing(2);
                    for name in tenants {
                        if !filter_lower.is_empty() && !name.to_lowercase().contains(&filter_lower)
                        {
                            continue;
                        }

                        // Inline edit form for this tenant
                        if let Some(ref form) = state.adding_tenant {
                            if form.editing.as_deref() == Some(name.as_str()) {
                                list =
                                    list.push(render_inline_tenant_form(form, on_message, space_s));
                                continue;
                            }
                        }

                        let is_selected = state.selected_tenant.as_deref() == Some(name.as_str());
                        let card = render_tenant_card(name, is_selected, server_index, on_message);
                        list = list.push(card);
                    }

                    // Inline add form at bottom
                    if let Some(ref form) = state.adding_tenant {
                        if form.editing.is_none() {
                            list = list.push(render_inline_tenant_form(form, on_message, space_s));
                        }
                    }

                    widget::scrollable(list)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .into()
                }
                _ => {
                    // Still show add form even when no tenants
                    if let Some(ref form) = state.adding_tenant {
                        if form.editing.is_none() {
                            return widget::column::with_capacity(2)
                                .push(header)
                                .push(render_inline_tenant_form(form, on_message, space_s))
                                .width(Length::Fill)
                                .height(Length::Fill)
                                .into();
                        }
                    }
                    widget::text::body("No tenants found").into()
                }
            }
        }
    };

    widget::column::with_capacity(2)
        .push(header)
        .push(content)
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

    // Custom header with add button that opens a dialog
    let filter_visible = state.database_filter_visible;
    let header_row = if filter_visible {
        let input = widget::text_input("Filter...", &state.database_filter)
            .on_input(move |s| on_message(BrowserMsg::UpdateFilter(ColumnLevel::Databases, s)))
            .width(Length::Fill)
            .size(14);
        let clear_btn = widget::button::icon(icon::from_name("edit-clear-symbolic"))
            .extra_small()
            .on_press(on_message(BrowserMsg::ClearFilter(ColumnLevel::Databases)));
        widget::row::with_capacity(2)
            .push(input)
            .push(clear_btn)
            .spacing(4)
            .align_y(Alignment::Center)
    } else {
        let title = widget::text::heading("Databases").width(Length::Fill);
        let search_btn = widget::button::icon(icon::from_name("system-search-symbolic"))
            .extra_small()
            .on_press(on_message(BrowserMsg::ToggleFilter(ColumnLevel::Databases)));
        let add_btn = widget::button::icon(icon::from_name("list-add-symbolic"))
            .extra_small()
            .on_press(on_message(BrowserMsg::OpenAddDatabaseDialog {
                server_index,
                tenant: tenant_owned.clone(),
            }));
        widget::row::with_capacity(3)
            .push(title)
            .push(search_btn)
            .push(add_btn)
            .spacing(4)
            .align_y(Alignment::Center)
    };

    let header = widget::column::with_capacity(2)
        .push(
            widget::container(header_row)
                .padding([spacing.space_xxs, spacing.space_xs])
                .width(Length::Fill),
        )
        .push(widget::divider::horizontal::default())
        .width(Length::Fill);

    let cache_key = format!("{server_index}:{tenant}");
    let filter_lower = state.database_filter.to_lowercase();

    let content: Element<'a, Message> = match &state.databases_status {
        LoadStatus::Idle => widget::text::body("Select a tenant").into(),
        LoadStatus::Loading => render_loading("Loading databases..."),
        LoadStatus::Error(e) => render_error(e),
        LoadStatus::Loaded => {
            let databases = state.databases_cache.get(&cache_key);
            match databases {
                Some(databases) if !databases.is_empty() => {
                    let mut list = widget::column::with_capacity(databases.len()).spacing(2);
                    for name in databases {
                        if !filter_lower.is_empty() && !name.to_lowercase().contains(&filter_lower)
                        {
                            continue;
                        }
                        let is_selected = state.selected_database.as_deref() == Some(name.as_str());
                        let card = render_database_card(
                            name,
                            is_selected,
                            server_index,
                            tenant_owned.clone(),
                            on_message,
                        );
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

    let filter_visible = state.collection_filter_visible;
    let header_row = if filter_visible {
        let input = widget::text_input("Filter...", &state.collection_filter)
            .on_input(move |s| on_message(BrowserMsg::UpdateFilter(ColumnLevel::Collections, s)))
            .width(Length::Fill)
            .size(14);
        let clear_btn = widget::button::icon(icon::from_name("edit-clear-symbolic"))
            .extra_small()
            .on_press(on_message(BrowserMsg::ClearFilter(
                ColumnLevel::Collections,
            )));
        widget::row::with_capacity(2)
            .push(input)
            .push(clear_btn)
            .spacing(4)
            .align_y(Alignment::Center)
    } else {
        let title = widget::text::heading("Collections").width(Length::Fill);
        let search_btn = widget::button::icon(icon::from_name("system-search-symbolic"))
            .extra_small()
            .on_press(on_message(BrowserMsg::ToggleFilter(
                ColumnLevel::Collections,
            )));
        let add_btn = widget::button::icon(icon::from_name("list-add-symbolic"))
            .extra_small()
            .on_press(on_message(BrowserMsg::OpenAddCollectionDialog {
                server_index,
                tenant: tenant_owned.clone(),
                database: database_owned.clone(),
            }));
        widget::row::with_capacity(3)
            .push(title)
            .push(search_btn)
            .push(add_btn)
            .spacing(4)
            .align_y(Alignment::Center)
    };

    let header = widget::column::with_capacity(2)
        .push(
            widget::container(header_row)
                .padding([spacing.space_xxs, spacing.space_xs])
                .width(Length::Fill),
        )
        .push(widget::divider::horizontal::default())
        .width(Length::Fill);

    let cache_key = format!("{server_index}:{tenant}:{database}");
    let filter_lower = state.collection_filter.to_lowercase();

    let content: Element<'a, Message> = match &state.collections_status {
        LoadStatus::Idle => widget::text::body("Select a database").into(),
        LoadStatus::Loading => render_loading("Loading collections..."),
        LoadStatus::Error(e) => render_error(e),
        LoadStatus::Loaded => {
            let collections = state.collections_cache.get(&cache_key);
            match collections {
                Some(collections) if !collections.is_empty() => {
                    let mut list = widget::column::with_capacity(collections.len()).spacing(2);
                    for collection in collections {
                        if !filter_lower.is_empty()
                            && !collection.name.to_lowercase().contains(&filter_lower)
                        {
                            continue;
                        }
                        let is_selected =
                            state.selected_collection.as_deref() == Some(collection.id.as_str());
                        let card = render_collection_card(
                            collection,
                            is_selected,
                            server_index,
                            tenant_owned.clone(),
                            database_owned.clone(),
                            on_message,
                        );
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

    let filter_visible = state.document_filter_visible;
    let header_row = if filter_visible {
        let input = widget::text_input("Filter...", &state.document_filter)
            .on_input(move |s| on_message(BrowserMsg::UpdateFilter(ColumnLevel::Documents, s)))
            .width(Length::Fill)
            .size(14);
        let clear_btn = widget::button::icon(icon::from_name("edit-clear-symbolic"))
            .extra_small()
            .on_press(on_message(BrowserMsg::ClearFilter(ColumnLevel::Documents)));
        widget::row::with_capacity(2)
            .push(input)
            .push(clear_btn)
            .spacing(4)
            .align_y(Alignment::Center)
    } else {
        let title = widget::text::heading("Documents").width(Length::Fill);
        let search_btn = widget::button::icon(icon::from_name("system-search-symbolic"))
            .extra_small()
            .on_press(on_message(BrowserMsg::ToggleFilter(ColumnLevel::Documents)));
        widget::row::with_capacity(2)
            .push(title)
            .push(search_btn)
            .spacing(4)
            .align_y(Alignment::Center)
    };

    let header = widget::column::with_capacity(2)
        .push(
            widget::container(header_row)
                .padding([spacing.space_xxs, spacing.space_xs])
                .width(Length::Fill),
        )
        .push(widget::divider::horizontal::default())
        .width(Length::Fill);

    let cache_key = format!("{server_index}:{tenant}:{database}:{collection_id}");
    let filter_lower = state.document_filter.to_lowercase();

    let content: Element<'a, Message> = match &state.documents_status {
        LoadStatus::Idle => widget::text::body("Select a collection").into(),
        LoadStatus::Loading => render_loading("Loading documents..."),
        LoadStatus::Error(e) => render_error(e),
        LoadStatus::Loaded => {
            let documents = state.documents_cache.get(&cache_key);
            match documents {
                Some(documents) if !documents.is_empty() => {
                    let mut list = widget::column::with_capacity(documents.len()).spacing(2);
                    for doc in documents {
                        if !filter_lower.is_empty() {
                            let id_match = doc.id.to_lowercase().contains(&filter_lower);
                            let content_match = doc
                                .document
                                .as_ref()
                                .is_some_and(|s| s.to_lowercase().contains(&filter_lower));
                            if !id_match && !content_match {
                                continue;
                            }
                        }
                        let is_selected = state
                            .selected_document
                            .as_ref()
                            .is_some_and(|d| d.id == doc.id);
                        let card = render_document_card(
                            doc,
                            is_selected,
                            server_index,
                            tenant_owned.clone(),
                            database_owned.clone(),
                            collection_id_owned.clone(),
                            on_message,
                        );
                        list = list.push(card);
                    }
                    widget::scrollable(list)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .into()
                }
                _ => widget::text::body("No documents found").into(),
            }
        }
    };

    widget::column::with_capacity(2)
        .push(header)
        .push(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

// ── Shared status renderers ──────────────────────────────────────────────────

fn render_loading<'a, Message: 'static>(text: &str) -> Element<'a, Message> {
    widget::container(widget::text::body(text.to_string()))
        .width(Length::Fill)
        .center(Length::Fill)
        .into()
}

fn render_error<'a, Message: 'static>(error: &str) -> Element<'a, Message> {
    widget::container(
        widget::column::with_capacity(2)
            .push(icon::from_name("dialog-error-symbolic").size(24))
            .push(widget::text::caption(error.to_string()).class(cosmic::style::Text::Accent))
            .spacing(8)
            .align_x(Alignment::Center),
    )
    .width(Length::Fill)
    .center(Length::Fill)
    .into()
}

// ── Context menu helpers ─────────────────────────────────────────────────────

/// Build a single context menu tree item (a clickable button) with the given label and message.
fn menu_item<Message: Clone + 'static>(label: &str, msg: Message) -> menu::Tree<Message> {
    menu::Tree::from(Element::from(
        menu::menu_button(vec![widget::text(label.to_string()).into()]).on_press(msg),
    ))
}

/// Build a context menu divider.
fn menu_divider<Message: Clone + 'static>() -> menu::Tree<Message> {
    menu::Tree::from(Element::from(widget::divider::horizontal::light()))
}

// ── Card renderers (using libcosmic context_menu) ────────────────────────────

/// Renders a server card with right-click context menu.
fn render_server_card<'a, Message: Clone + 'static>(
    config: &ServerConfig,
    index: usize,
    is_selected: bool,
    on_message: impl Fn(BrowserMsg) -> Message + Copy + 'a,
) -> Element<'a, Message> {
    let name = config.name.clone();
    let url = config.server_url.clone();

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

    let card = widget::container(card_content)
        .padding(10)
        .width(Length::Fill)
        .class(container_class);

    let select_msg = on_message(BrowserMsg::SelectServer(index));

    let menu_trees = vec![
        menu_item("Edit", on_message(BrowserMsg::StartEditServer { index })),
        menu_divider(),
        menu_item(
            "Delete",
            on_message(BrowserMsg::RequestDeleteServer { index }),
        ),
    ];

    context_menu(
        widget::mouse_area(card).on_press(select_msg),
        Some(menu_trees),
    )
    .into()
}

/// Renders a tenant card with right-click context menu (Edit, Delete).
fn render_tenant_card<'a, Message: Clone + 'static>(
    name: &str,
    is_selected: bool,
    server_index: usize,
    on_message: impl Fn(BrowserMsg) -> Message + Copy + 'a,
) -> Element<'a, Message> {
    let name_owned = name.to_string();

    let row = widget::row::with_capacity(3)
        .push(icon::from_name("system-users-symbolic").size(16))
        .push(
            widget::text::body(name_owned.clone())
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

    let card = widget::container(row)
        .padding([6, 10])
        .width(Length::Fill)
        .class(container_class);

    let select_msg = on_message(BrowserMsg::SelectTenant(name_owned.clone()));
    let set_ctx = on_message(BrowserMsg::SetMenuContext(MenuContext::Tenant {
        server_index,
        name: name_owned,
    }));

    let menu_trees = vec![menu_item(
        "Edit",
        on_message(BrowserMsg::EditTenantFromMenu),
    )];

    // Use mouse_area for left-click select and right-click to set context
    let area = widget::mouse_area(card)
        .on_press(select_msg)
        .on_right_press(set_ctx);

    context_menu(area, Some(menu_trees)).into()
}

/// Renders a database card with right-click context menu.
fn render_database_card<'a, Message: Clone + 'static>(
    name: &str,
    is_selected: bool,
    server_index: usize,
    tenant: String,
    on_message: impl Fn(BrowserMsg) -> Message + Copy + 'a,
) -> Element<'a, Message> {
    let db_name = name.to_string();

    let row = widget::row::with_capacity(3)
        .push(icon::from_name("drive-harddisk-symbolic").size(16))
        .push(
            widget::text::body(db_name.clone())
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

    let card = widget::container(row)
        .padding([6, 10])
        .width(Length::Fill)
        .class(container_class);

    let select_msg = on_message(BrowserMsg::SelectDatabase(db_name.clone()));
    let set_ctx = on_message(BrowserMsg::SetMenuContext(MenuContext::Database {
        server_index,
        tenant,
        name: db_name,
    }));

    let menu_trees = vec![menu_item(
        "Delete",
        on_message(BrowserMsg::DeleteDatabaseFromMenu),
    )];

    let area = widget::mouse_area(card)
        .on_press(select_msg)
        .on_right_press(set_ctx);

    context_menu(area, Some(menu_trees)).into()
}

/// Renders a collection card with right-click context menu.
fn render_collection_card<'a, Message: Clone + 'static>(
    collection: &Collection,
    is_selected: bool,
    server_index: usize,
    tenant: String,
    database: String,
    on_message: impl Fn(BrowserMsg) -> Message + Copy + 'a,
) -> Element<'a, Message> {
    let collection_id = collection.id.clone();
    let collection_name = collection.name.clone();
    let metadata_count = collection.metadata.as_ref().map(|m| m.len()).unwrap_or(0);

    let header_row = widget::row::with_capacity(3)
        .push(icon::from_name("folder-symbolic").size(16))
        .push(
            widget::text::body(collection_name.clone())
                .width(Length::Fill)
                .class(cosmic::style::Text::Default),
        )
        .push(icon::from_name("go-next-symbolic").size(12))
        .align_y(Alignment::Center)
        .spacing(8);

    let metadata_info = if metadata_count > 0 {
        widget::text::caption(format!("{metadata_count} metadata fields"))
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

    let card = widget::container(card_content)
        .padding(10)
        .width(Length::Fill)
        .class(container_class);

    let select_msg = on_message(BrowserMsg::SelectCollection(collection_id.clone()));
    let set_ctx = on_message(BrowserMsg::SetMenuContext(MenuContext::Collection {
        server_index,
        tenant,
        database,
        collection_id,
        collection_name,
    }));

    let menu_trees = vec![menu_item(
        "Delete",
        on_message(BrowserMsg::DeleteCollectionFromMenu),
    )];

    let area = widget::mouse_area(card)
        .on_press(select_msg)
        .on_right_press(set_ctx);

    context_menu(area, Some(menu_trees)).into()
}

/// Renders a document card with right-click context menu.
fn render_document_card<'a, Message: Clone + 'static>(
    doc: &Document,
    is_selected: bool,
    server_index: usize,
    tenant: String,
    database: String,
    collection_id: String,
    on_message: impl Fn(BrowserMsg) -> Message + Copy + 'a,
) -> Element<'a, Message> {
    let doc_id = doc.id.clone();

    let content_preview = doc
        .document
        .as_ref()
        .map(|s| truncate_str(s, 80))
        .unwrap_or_else(|| "[No content]".to_string());

    let metadata_count = doc.metadata.as_ref().map(|m| m.len()).unwrap_or(0);

    let header_row = widget::row::with_capacity(2)
        .push(icon::from_name("text-x-generic-symbolic").size(16))
        .push(
            widget::text::body(doc_id.clone())
                .width(Length::Fill)
                .class(cosmic::style::Text::Default),
        )
        .align_y(Alignment::Center)
        .spacing(8);

    let content_text = widget::text::caption(content_preview).class(cosmic::style::Text::Default);
    let metadata_info = widget::text::caption(format!("{metadata_count} metadata fields"))
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

    let card = widget::container(card_content)
        .padding(10)
        .width(Length::Fill)
        .class(container_class);

    let select_msg = on_message(BrowserMsg::SelectDocument(doc_id.clone()));
    let set_ctx = on_message(BrowserMsg::SetMenuContext(MenuContext::Document {
        server_index,
        tenant,
        database,
        collection_id,
        document_id: doc_id,
    }));

    let menu_trees = vec![menu_item(
        "Delete",
        on_message(BrowserMsg::DeleteDocumentFromMenu),
    )];

    let area = widget::mouse_area(card)
        .on_press(select_msg)
        .on_right_press(set_ctx);

    context_menu(area, Some(menu_trees)).into()
}

// ── Inline forms ─────────────────────────────────────────────────────────────

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

/// Renders an inline tenant form for add/edit.
fn render_inline_tenant_form<'a, Message: Clone + 'static>(
    form: &'a AddTenantForm,
    on_message: impl Fn(BrowserMsg) -> Message + Copy + 'a,
    space_s: u16,
) -> Element<'a, Message> {
    let is_editing = form.editing.is_some();
    let title = if is_editing {
        "Edit Tenant"
    } else {
        "Add Tenant"
    };

    let mut content = widget::column::with_capacity(5).spacing(4);

    content = content.push(widget::text::body(title).class(cosmic::style::Text::Accent));
    content = content.push(widget::text::caption("Name"));
    content = content.push(
        widget::text_input("Tenant name", &form.name)
            .on_input(move |s| on_message(BrowserMsg::TenantFormNameChanged(s)))
            .on_submit(move |_| on_message(BrowserMsg::SaveTenant))
            .width(Length::Fill)
            .padding(4),
    );

    let button_row = widget::row::with_capacity(2)
        .push(
            widget::button::text("Cancel")
                .class(cosmic::theme::Button::Standard)
                .padding([4, 8])
                .on_press(on_message(BrowserMsg::CancelTenantForm)),
        )
        .push(
            widget::button::suggested("Save")
                .padding([4, 8])
                .on_press(on_message(BrowserMsg::SaveTenant)),
        )
        .spacing(4);
    content = content.push(button_row);

    widget::container(content)
        .padding(space_s)
        .width(Length::Fill)
        .class(cosmic::style::Container::Card)
        .into()
}

// ── Dialogs ──────────────────────────────────────────────────────────────────

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
                    "Tenant '{tenant}' doesn't exist on the server. Would you like to create it?"
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
                BrowserDialog::AddDatabase { .. } => ("Add Database", "Database name"),
                BrowserDialog::AddCollection { .. } => ("Add Collection", "Collection name"),
                BrowserDialog::ConfirmCreateTenant { .. } => unreachable!(),
            };

            let value = match dialog {
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
