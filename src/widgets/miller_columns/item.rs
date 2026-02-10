// SPDX-License-Identifier: MPL-2.0

//! Item types for Miller columns widget.

/// Unique identifier for items in the Miller columns.
pub type ItemId = String;

/// Represents whether an item can have children or is a leaf node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MillerItemType {
    /// Item can have children; selecting it may populate the next column.
    #[default]
    Branch,
    /// Item is a leaf; selecting it emits a selection event but doesn't expand.
    Leaf,
}

impl MillerItemType {
    /// Returns true if this item type can have children.
    pub fn is_branch(&self) -> bool {
        matches!(self, MillerItemType::Branch)
    }

    /// Returns true if this item type is a leaf (no children).
    pub fn is_leaf(&self) -> bool {
        matches!(self, MillerItemType::Leaf)
    }
}

/// Represents a single item in the Miller columns.
///
/// Generic over `D` which is the custom data type associated with each item.
#[derive(Debug, Clone)]
pub struct MillerItem<D> {
    /// Unique identifier for this item.
    pub id: ItemId,
    /// Display label for the item.
    pub label: String,
    /// Whether this item can have children (branch) or not (leaf).
    pub item_type: MillerItemType,
    /// Custom data associated with this item.
    pub data: D,
}

impl<D> MillerItem<D> {
    /// Creates a new branch item (can have children).
    pub fn branch(id: impl Into<String>, label: impl Into<String>, data: D) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            item_type: MillerItemType::Branch,
            data,
        }
    }

    /// Creates a new leaf item (no children).
    pub fn leaf(id: impl Into<String>, label: impl Into<String>, data: D) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            item_type: MillerItemType::Leaf,
            data,
        }
    }

    /// Returns true if this item can have children.
    pub fn is_branch(&self) -> bool {
        self.item_type.is_branch()
    }

    /// Returns true if this item is a leaf (no children).
    pub fn is_leaf(&self) -> bool {
        self.item_type.is_leaf()
    }
}

impl<D: Default> MillerItem<D> {
    /// Creates a new branch item with default data.
    pub fn branch_default(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self::branch(id, label, D::default())
    }

    /// Creates a new leaf item with default data.
    pub fn leaf_default(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self::leaf(id, label, D::default())
    }
}
