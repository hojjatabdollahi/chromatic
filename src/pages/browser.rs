// SPDX-License-Identifier: MPL-2.0

//! Browser page using Miller columns for hierarchical navigation.
//!
//! Column hierarchy:
//! 1. Server configs (+ Add New)
//! 2. Tenants (+ Add New)
//! 3. Databases (+ Add New)
//! 4. Collections (+ Add New)
//! 5. Documents
//! 6. Document preview

use crate::api::{Collection, Document};
use crate::config::ServerConfig;
use crate::widgets::miller_columns::{MillerItem, MillerItemType, MillerMessage, MillerState};
use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::widget::{self, icon};
use std::collections::HashMap;

/// The type of data represented by a browser item.
#[derive(Debug, Clone)]
pub enum BrowserData {
    /// A server configuration
    Server { index: usize, config: ServerConfig },
    /// Add new server action
    AddServer,
    /// A tenant
    Tenant { server_index: usize, name: String },
    /// Add new tenant action
    AddTenant { server_index: usize },
    /// A database
    Database {
        server_index: usize,
        tenant: String,
        name: String,
    },
    /// Add new database action
    AddDatabase { server_index: usize, tenant: String },
    /// A collection
    Collection {
        server_index: usize,
        tenant: String,
        database: String,
        collection: Collection,
    },
    /// Add new collection action
    AddCollection {
        server_index: usize,
        tenant: String,
        database: String,
    },
    /// A document
    Document {
        server_index: usize,
        tenant: String,
        database: String,
        collection_id: String,
        document: Document,
    },
    /// Document preview
    DocumentPreview { document: Document },
}

/// State specific to the browser page.
#[derive(Debug, Clone, Default)]
pub struct BrowserState {
    /// The Miller columns state
    pub miller: MillerState<BrowserData>,
    /// Cached tenants per server (server_index -> tenant names)
    pub tenants_cache: HashMap<usize, Vec<String>>,
    /// Cached databases per server+tenant (key: "server_idx:tenant" -> database names)
    pub databases_cache: HashMap<String, Vec<String>>,
    /// Cached collections per server+tenant+database
    pub collections_cache: HashMap<String, Vec<Collection>>,
    /// Cached documents per collection
    pub documents_cache: HashMap<String, Vec<Document>>,
    /// Currently selected document for preview
    pub selected_document: Option<Document>,
    /// Dialog state for adding new items
    pub dialog: Option<BrowserDialog>,
}

/// Dialog types for adding new items.
#[derive(Debug, Clone)]
pub enum BrowserDialog {
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
}

impl BrowserState {
    /// Creates a new browser state with the given server configs.
    pub fn new(servers: &[ServerConfig]) -> Self {
        let roots = Self::build_server_items(servers);
        Self {
            miller: MillerState::new(roots),
            tenants_cache: HashMap::new(),
            databases_cache: HashMap::new(),
            collections_cache: HashMap::new(),
            documents_cache: HashMap::new(),
            selected_document: None,
            dialog: None,
        }
    }

    /// Rebuilds the root items from server configs.
    pub fn refresh_servers(&mut self, servers: &[ServerConfig]) {
        self.miller.set_roots(Self::build_server_items(servers));
    }

    /// Builds Miller items for server configs.
    fn build_server_items(servers: &[ServerConfig]) -> Vec<MillerItem<BrowserData>> {
        let mut items = Vec::with_capacity(servers.len() + 1);

        // Add server items
        for (index, config) in servers.iter().enumerate() {
            items.push(MillerItem::branch(
                format!("server:{}", index),
                &config.name,
                BrowserData::Server {
                    index,
                    config: config.clone(),
                },
            ));
        }

        // Add "Add New Server" item
        items.push(MillerItem::leaf(
            "add:server",
            "+ Add Server",
            BrowserData::AddServer,
        ));

        items
    }

    /// Sets tenants for a server.
    pub fn set_tenants(&mut self, server_index: usize, tenants: Vec<String>) {
        self.tenants_cache.insert(server_index, tenants.clone());

        let items = Self::build_tenant_items(server_index, &tenants);
        self.miller
            .set_children(format!("server:{}", server_index), items);
    }

    /// Sets loading state for tenants.
    pub fn set_tenants_loading(&mut self, server_index: usize) {
        self.miller.set_loading(&format!("server:{}", server_index));
    }

    /// Sets error state for tenants.
    pub fn set_tenants_error(&mut self, server_index: usize, error: String) {
        self.miller
            .set_error(format!("server:{}", server_index), error);
    }

    /// Builds Miller items for tenants.
    fn build_tenant_items(server_index: usize, tenants: &[String]) -> Vec<MillerItem<BrowserData>> {
        let mut items = Vec::with_capacity(tenants.len() + 1);

        for name in tenants {
            items.push(MillerItem::branch(
                format!("tenant:{}:{}", server_index, name),
                name,
                BrowserData::Tenant {
                    server_index,
                    name: name.clone(),
                },
            ));
        }

        // Add "Add New Tenant" item
        items.push(MillerItem::leaf(
            format!("add:tenant:{}", server_index),
            "+ Add Tenant",
            BrowserData::AddTenant { server_index },
        ));

        items
    }

    /// Sets databases for a tenant.
    pub fn set_databases(&mut self, server_index: usize, tenant: &str, databases: Vec<String>) {
        let cache_key = format!("{}:{}", server_index, tenant);
        self.databases_cache.insert(cache_key, databases.clone());

        let items = Self::build_database_items(server_index, tenant, &databases);
        self.miller
            .set_children(format!("tenant:{}:{}", server_index, tenant), items);
    }

    /// Sets loading state for databases.
    pub fn set_databases_loading(&mut self, server_index: usize, tenant: &str) {
        self.miller
            .set_loading(&format!("tenant:{}:{}", server_index, tenant));
    }

    /// Sets error state for databases.
    pub fn set_databases_error(&mut self, server_index: usize, tenant: &str, error: String) {
        self.miller
            .set_error(format!("tenant:{}:{}", server_index, tenant), error);
    }

    /// Builds Miller items for databases.
    fn build_database_items(
        server_index: usize,
        tenant: &str,
        databases: &[String],
    ) -> Vec<MillerItem<BrowserData>> {
        let mut items = Vec::with_capacity(databases.len() + 1);

        for name in databases {
            items.push(MillerItem::branch(
                format!("database:{}:{}:{}", server_index, tenant, name),
                name,
                BrowserData::Database {
                    server_index,
                    tenant: tenant.to_string(),
                    name: name.clone(),
                },
            ));
        }

        // Add "Add New Database" item
        items.push(MillerItem::leaf(
            format!("add:database:{}:{}", server_index, tenant),
            "+ Add Database",
            BrowserData::AddDatabase {
                server_index,
                tenant: tenant.to_string(),
            },
        ));

        items
    }

    /// Sets collections for a database.
    pub fn set_collections(
        &mut self,
        server_index: usize,
        tenant: &str,
        database: &str,
        collections: Vec<Collection>,
    ) {
        let cache_key = format!("{}:{}:{}", server_index, tenant, database);
        self.collections_cache
            .insert(cache_key, collections.clone());

        let items = Self::build_collection_items(server_index, tenant, database, &collections);
        self.miller.set_children(
            format!("database:{}:{}:{}", server_index, tenant, database),
            items,
        );
    }

    /// Sets loading state for collections.
    pub fn set_collections_loading(&mut self, server_index: usize, tenant: &str, database: &str) {
        self.miller.set_loading(&format!(
            "database:{}:{}:{}",
            server_index, tenant, database
        ));
    }

    /// Sets error state for collections.
    pub fn set_collections_error(
        &mut self,
        server_index: usize,
        tenant: &str,
        database: &str,
        error: String,
    ) {
        self.miller.set_error(
            format!("database:{}:{}:{}", server_index, tenant, database),
            error,
        );
    }

    /// Builds Miller items for collections.
    fn build_collection_items(
        server_index: usize,
        tenant: &str,
        database: &str,
        collections: &[Collection],
    ) -> Vec<MillerItem<BrowserData>> {
        let mut items = Vec::with_capacity(collections.len() + 1);

        for collection in collections {
            items.push(MillerItem::branch(
                format!(
                    "collection:{}:{}:{}:{}",
                    server_index, tenant, database, collection.id
                ),
                &collection.name,
                BrowserData::Collection {
                    server_index,
                    tenant: tenant.to_string(),
                    database: database.to_string(),
                    collection: collection.clone(),
                },
            ));
        }

        // Add "Add New Collection" item
        items.push(MillerItem::leaf(
            format!("add:collection:{}:{}:{}", server_index, tenant, database),
            "+ Add Collection",
            BrowserData::AddCollection {
                server_index,
                tenant: tenant.to_string(),
                database: database.to_string(),
            },
        ));

        items
    }

    /// Sets documents for a collection.
    pub fn set_documents(
        &mut self,
        server_index: usize,
        tenant: &str,
        database: &str,
        collection_id: &str,
        documents: Vec<Document>,
    ) {
        let cache_key = format!("{}:{}:{}:{}", server_index, tenant, database, collection_id);
        self.documents_cache.insert(cache_key, documents.clone());

        let items =
            Self::build_document_items(server_index, tenant, database, collection_id, &documents);
        self.miller.set_children(
            format!(
                "collection:{}:{}:{}:{}",
                server_index, tenant, database, collection_id
            ),
            items,
        );
    }

    /// Sets loading state for documents.
    pub fn set_documents_loading(
        &mut self,
        server_index: usize,
        tenant: &str,
        database: &str,
        collection_id: &str,
    ) {
        self.miller.set_loading(&format!(
            "collection:{}:{}:{}:{}",
            server_index, tenant, database, collection_id
        ));
    }

    /// Sets error state for documents.
    pub fn set_documents_error(
        &mut self,
        server_index: usize,
        tenant: &str,
        database: &str,
        collection_id: &str,
        error: String,
    ) {
        self.miller.set_error(
            format!(
                "collection:{}:{}:{}:{}",
                server_index, tenant, database, collection_id
            ),
            error,
        );
    }

    /// Builds Miller items for documents.
    fn build_document_items(
        server_index: usize,
        tenant: &str,
        database: &str,
        collection_id: &str,
        documents: &[Document],
    ) -> Vec<MillerItem<BrowserData>> {
        documents
            .iter()
            .map(|doc| {
                // Use a truncated preview of document content as label
                let label = doc
                    .document
                    .as_ref()
                    .map(|s| {
                        if s.len() > 40 {
                            format!("{}...", &s[..40])
                        } else {
                            s.clone()
                        }
                    })
                    .unwrap_or_else(|| doc.id.clone());

                MillerItem::leaf(
                    format!(
                        "document:{}:{}:{}:{}:{}",
                        server_index, tenant, database, collection_id, doc.id
                    ),
                    label,
                    BrowserData::Document {
                        server_index,
                        tenant: tenant.to_string(),
                        database: database.to_string(),
                        collection_id: collection_id.to_string(),
                        document: doc.clone(),
                    },
                )
            })
            .collect()
    }
}

/// Messages specific to the browser.
#[derive(Debug, Clone)]
pub enum BrowserMsg {
    /// Miller column message
    Miller(MillerMessage<BrowserData>),
    /// Tenants loaded for a server
    TenantsLoaded {
        server_index: usize,
        result: Result<Vec<String>, String>,
    },
    /// Databases loaded for a tenant
    DatabasesLoaded {
        server_index: usize,
        tenant: String,
        result: Result<Vec<String>, String>,
    },
    /// Collections loaded for a database
    CollectionsLoaded {
        server_index: usize,
        tenant: String,
        database: String,
        result: Result<Vec<Collection>, String>,
    },
    /// Documents loaded for a collection
    DocumentsLoaded {
        server_index: usize,
        tenant: String,
        database: String,
        collection_id: String,
        result: Result<Vec<Document>, String>,
    },
    /// Dialog input changed
    DialogInputChanged(String),
    /// Dialog confirmed
    DialogConfirm,
    /// Dialog cancelled
    DialogCancel,
    /// Server created
    ServerCreated,
    /// Tenant created
    TenantCreated {
        server_index: usize,
        tenant: String,
        result: Result<(), String>,
    },
    /// Database created
    DatabaseCreated {
        server_index: usize,
        tenant: String,
        database: String,
        result: Result<(), String>,
    },
    /// Collection created
    CollectionCreated {
        server_index: usize,
        tenant: String,
        database: String,
        result: Result<Collection, String>,
    },
}

/// Renders the browser view.
pub fn view<'a, Message: Clone + 'static>(
    state: &'a BrowserState,
    on_message: impl Fn(BrowserMsg) -> Message + Copy + 'a,
    space_s: u16,
    space_m: u16,
) -> Element<'a, Message> {
    use crate::widgets::MillerColumns;

    let miller_view: Element<'a, Message> = MillerColumns::new(&state.miller, move |msg| {
        on_message(BrowserMsg::Miller(msg))
    })
    .column_width(Length::Fixed(220.0))
    .spacing(space_s)
    .item_view(|item, is_selected| render_browser_item(item, is_selected))
    .into();

    // If we have a selected document, show the preview
    let content: Element<'a, Message> = if let Some(ref doc) = state.selected_document {
        widget::row::with_capacity(2)
            .push(miller_view)
            .push(render_document_preview(doc, space_s))
            .spacing(space_m)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        miller_view
    };

    // Wrap in dialog if one is open
    if let Some(ref dialog) = state.dialog {
        render_dialog(content, dialog, on_message, space_s)
    } else {
        content
    }
}

/// Renders a single browser item.
fn render_browser_item<'a, Message: 'static>(
    item: &MillerItem<BrowserData>,
    is_selected: bool,
) -> Element<'a, Message> {
    let (icon_name, label_style) = match &item.data {
        BrowserData::Server { .. } => ("network-server-symbolic", false),
        BrowserData::AddServer => ("list-add-symbolic", true),
        BrowserData::Tenant { .. } => ("system-users-symbolic", false),
        BrowserData::AddTenant { .. } => ("list-add-symbolic", true),
        BrowserData::Database { .. } => ("drive-harddisk-symbolic", false),
        BrowserData::AddDatabase { .. } => ("list-add-symbolic", true),
        BrowserData::Collection { .. } => ("folder-symbolic", false),
        BrowserData::AddCollection { .. } => ("list-add-symbolic", true),
        BrowserData::Document { .. } => ("text-x-generic-symbolic", false),
        BrowserData::DocumentPreview { .. } => ("text-x-generic-symbolic", false),
    };

    let has_children = item.item_type == MillerItemType::Branch;

    let label = item.label.clone();
    let mut row = widget::row::with_capacity(3)
        .push(icon::from_name(icon_name).size(16))
        .push(
            widget::text::body(label)
                .width(Length::Fill)
                .class(if label_style {
                    cosmic::style::Text::Accent
                } else {
                    cosmic::style::Text::Default
                }),
        )
        .align_y(Alignment::Center)
        .spacing(8);

    if has_children {
        row = row.push(icon::from_name("go-next-symbolic").size(12));
    }

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

/// Renders the document preview panel.
fn render_document_preview<'a, Message: 'static>(
    doc: &'a Document,
    space_s: u16,
) -> Element<'a, Message> {
    let mut content = widget::column::with_capacity(6).spacing(space_s);

    // Document ID
    content = content.push(widget::text::title4("Document ID"));
    content = content.push(
        widget::container(widget::text::body(&doc.id))
            .padding(space_s)
            .width(Length::Fill)
            .class(cosmic::style::Container::Card),
    );

    // Document content
    content = content.push(widget::text::title4("Content"));
    let doc_content = doc.document.as_deref().unwrap_or("[No content]");
    content = content.push(
        widget::container(widget::text::body(doc_content))
            .padding(space_s)
            .width(Length::Fill)
            .class(cosmic::style::Container::Card),
    );

    // Metadata
    if let Some(ref metadata) = doc.metadata {
        if !metadata.is_empty() {
            content = content.push(widget::text::title4("Metadata"));

            let mut metadata_col = widget::column::with_capacity(metadata.len()).spacing(4);
            for (key, value) in metadata {
                let row = widget::row::with_capacity(2)
                    .push(widget::text::body(format!("{}:", key)).width(Length::Fixed(120.0)))
                    .push(widget::text::caption(value.to_string()))
                    .spacing(8);
                metadata_col = metadata_col.push(row);
            }

            content = content.push(
                widget::container(metadata_col)
                    .padding(space_s)
                    .width(Length::Fill)
                    .class(cosmic::style::Container::Card),
            );
        }
    }

    widget::scrollable(content)
        .width(Length::Fixed(350.0))
        .height(Length::Fill)
        .into()
}

/// Renders a dialog for adding new items.
fn render_dialog<'a, Message: Clone + 'static>(
    background: Element<'a, Message>,
    dialog: &'a BrowserDialog,
    on_message: impl Fn(BrowserMsg) -> Message + Copy + 'a,
    space_s: u16,
) -> Element<'a, Message> {
    let (title, placeholder) = match dialog {
        BrowserDialog::AddServer { .. } => ("Add Server", "Server name"),
        BrowserDialog::AddTenant { .. } => ("Add Tenant", "Tenant name"),
        BrowserDialog::AddDatabase { .. } => ("Add Database", "Database name"),
        BrowserDialog::AddCollection { .. } => ("Add Collection", "Collection name"),
    };

    let value = match dialog {
        BrowserDialog::AddServer { name } => name,
        BrowserDialog::AddTenant { name, .. } => name,
        BrowserDialog::AddDatabase { name, .. } => name,
        BrowserDialog::AddCollection { name, .. } => name,
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

    let dialog_widget: Element<'a, Message> =
        widget::dialog().title(title).control(dialog_content).into();

    widget::popover(background)
        .modal(true)
        .popup(dialog_widget)
        .into()
}
