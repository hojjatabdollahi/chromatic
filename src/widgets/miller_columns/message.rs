// SPDX-License-Identifier: MPL-2.0

//! Messages emitted by the Miller columns widget.

use super::item::{ItemId, MillerItem};
use super::state::SelectionPath;

/// Messages emitted by the Miller columns widget.
///
/// These messages should be wrapped by the parent's message type and handled
/// in the parent's update function.
#[derive(Debug, Clone)]
pub enum MillerMessage<D: Clone> {
    /// User selected an item at the given column index.
    ///
    /// Contains the full path from root to the selected item.
    Select {
        /// The column index where the selection occurred (0-indexed).
        column: usize,
        /// The full selection path from root to the selected item.
        path: SelectionPath,
        /// The item that was selected.
        item: MillerItem<D>,
    },

    /// User activated a leaf item (e.g., double-click, Enter key).
    ///
    /// This is only emitted for leaf items, not branches.
    Activate {
        /// The full selection path to the activated item.
        path: SelectionPath,
        /// The item that was activated.
        item: MillerItem<D>,
    },

    /// Children are needed for a branch item.
    ///
    /// The parent should fetch children and call `state.set_children()`.
    NeedChildren {
        /// The selection path to the parent item.
        parent_path: SelectionPath,
        /// The ID of the parent item that needs children loaded.
        parent_id: ItemId,
    },

    /// Scroll position changed in a column.
    Scroll {
        /// The column index where scrolling occurred.
        column: usize,
        /// The new scroll offset.
        offset: f32,
    },
}
