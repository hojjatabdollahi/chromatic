// SPDX-License-Identifier: MPL-2.0

//! Miller columns widget for hierarchical data navigation.
//!
//! Miller columns (also known as cascading lists) are a UI pattern for navigating
//! tree-like hierarchical structures. Selecting an item in one column populates
//! the next column with its children, allowing multiple levels to be visible
//! at once.
//!
//! # Example
//!
//! ```ignore
//! use crate::widgets::miller_columns::{MillerColumns, MillerItem, MillerState, MillerMessage};
//!
//! // In your app state
//! struct AppModel {
//!     miller_state: MillerState<MyData>,
//! }
//!
//! // In your message enum
//! enum Message {
//!     Miller(MillerMessage<MyData>),
//! }
//!
//! // In your view function
//! fn view(&self) -> Element<'_, Message> {
//!     MillerColumns::new(&self.miller_state, Message::Miller)
//!         .column_width(Length::Fixed(200.0))
//!         .max_columns(4)
//!         .into()
//! }
//!
//! // In your update function
//! fn update(&mut self, message: Message) -> Task<Message> {
//!     match message {
//!         Message::Miller(miller_msg) => match miller_msg {
//!             MillerMessage::Select { path, item, .. } => {
//!                 self.miller_state.select(path);
//!                 if item.is_branch() {
//!                     // Trigger loading children
//!                     return self.load_children(item.id);
//!                 }
//!             }
//!             MillerMessage::Activate { item, .. } => {
//!                 // Handle leaf activation
//!             }
//!             MillerMessage::NeedChildren { parent_id, .. } => {
//!                 self.miller_state.set_loading(&parent_id);
//!                 return self.load_children(parent_id);
//!             }
//!             _ => {}
//!         }
//!     }
//!     Task::none()
//! }
//! ```

mod item;
mod message;
mod state;
mod widget;

pub use item::{ItemId, MillerItem, MillerItemType};
pub use message::MillerMessage;
pub use state::{ColumnState, MillerState, SelectionPath};
pub use widget::MillerColumns;
