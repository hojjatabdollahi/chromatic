// SPDX-License-Identifier: MPL-2.0

//! State management for Miller columns widget.

use super::item::{ItemId, MillerItem};
use std::collections::HashMap;

/// Represents the current selection path through the columns.
///
/// Each element is the selected item's ID at that column depth.
/// An empty path means nothing is selected.
pub type SelectionPath = Vec<ItemId>;

/// Loading state for a column's children.
#[derive(Debug, Clone)]
pub enum ColumnState<D> {
    /// No children loaded yet for this selection.
    NotLoaded,
    /// Currently fetching children.
    Loading,
    /// Children loaded successfully.
    Loaded(Vec<MillerItem<D>>),
    /// Failed to load children.
    Error(String),
}

impl<D> Default for ColumnState<D> {
    fn default() -> Self {
        ColumnState::NotLoaded
    }
}

impl<D> ColumnState<D> {
    /// Returns true if children are currently being loaded.
    pub fn is_loading(&self) -> bool {
        matches!(self, ColumnState::Loading)
    }

    /// Returns true if children have been loaded successfully.
    pub fn is_loaded(&self) -> bool {
        matches!(self, ColumnState::Loaded(_))
    }

    /// Returns true if loading failed.
    pub fn is_error(&self) -> bool {
        matches!(self, ColumnState::Error(_))
    }

    /// Returns the loaded children, if any.
    pub fn children(&self) -> Option<&[MillerItem<D>]> {
        match self {
            ColumnState::Loaded(children) => Some(children),
            _ => None,
        }
    }

    /// Returns the error message, if any.
    pub fn error(&self) -> Option<&str> {
        match self {
            ColumnState::Error(e) => Some(e),
            _ => None,
        }
    }
}

/// State for the Miller columns widget.
///
/// This state is owned by the parent component and passed to the widget.
#[derive(Debug, Clone)]
pub struct MillerState<D> {
    /// Root items (first column).
    pub roots: Vec<MillerItem<D>>,
    /// Current selection path.
    pub selection: SelectionPath,
    /// Children for each selected branch, indexed by parent ID.
    pub children: HashMap<ItemId, ColumnState<D>>,
    /// Scroll offsets per column (indexed by column number).
    pub scroll_offsets: Vec<f32>,
}

impl<D: Clone> Default for MillerState<D> {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

impl<D: Clone> MillerState<D> {
    /// Creates a new Miller state with the given root items.
    pub fn new(roots: Vec<MillerItem<D>>) -> Self {
        Self {
            roots,
            selection: Vec::new(),
            children: HashMap::new(),
            scroll_offsets: Vec::new(),
        }
    }

    /// Sets the root items for the first column.
    pub fn set_roots(&mut self, roots: Vec<MillerItem<D>>) {
        self.roots = roots;
        // Clear selection and children when roots change
        self.selection.clear();
        self.children.clear();
        self.scroll_offsets.clear();
    }

    /// Sets the selection path.
    ///
    /// This updates which items are selected in each column.
    pub fn select(&mut self, path: SelectionPath) {
        self.selection = path;
        // Ensure scroll_offsets has enough entries
        while self.scroll_offsets.len() <= self.selection.len() {
            self.scroll_offsets.push(0.0);
        }
    }

    /// Selects an item at the given column index.
    ///
    /// This truncates the selection path at the column and appends the new item ID.
    pub fn select_at(&mut self, column: usize, item_id: ItemId) {
        // Truncate selection to the column
        self.selection.truncate(column);
        // Append the new selection
        self.selection.push(item_id);
        // Ensure scroll_offsets has enough entries
        while self.scroll_offsets.len() <= self.selection.len() {
            self.scroll_offsets.push(0.0);
        }
    }

    /// Clears the selection.
    pub fn clear_selection(&mut self) {
        self.selection.clear();
    }

    /// Provide children for a parent item (after loading).
    pub fn set_children(&mut self, parent_id: ItemId, children: Vec<MillerItem<D>>) {
        self.children
            .insert(parent_id, ColumnState::Loaded(children));
    }

    /// Mark children as loading for a parent item.
    pub fn set_loading(&mut self, parent_id: &ItemId) {
        self.children
            .insert(parent_id.clone(), ColumnState::Loading);
    }

    /// Mark children as error for a parent item.
    pub fn set_error(&mut self, parent_id: ItemId, error: String) {
        self.children.insert(parent_id, ColumnState::Error(error));
    }

    /// Gets the column state for a parent item.
    pub fn get_column_state(&self, parent_id: &ItemId) -> &ColumnState<D> {
        self.children
            .get(parent_id)
            .unwrap_or(&ColumnState::NotLoaded)
    }

    /// Gets the children for a parent item, if loaded.
    pub fn get_children(&self, parent_id: &ItemId) -> Option<&[MillerItem<D>]> {
        self.children.get(parent_id).and_then(|s| s.children())
    }

    /// Gets the currently selected item (the last item in the selection path).
    pub fn selected_item(&self) -> Option<&MillerItem<D>> {
        let path = &self.selection;
        if path.is_empty() {
            return None;
        }

        // Walk the path to find the selected item
        let mut current_items: &[MillerItem<D>] = &self.roots;
        let mut selected: Option<&MillerItem<D>> = None;

        for id in path {
            selected = current_items.iter().find(|item| &item.id == id);
            if let Some(item) = selected {
                if item.is_branch() {
                    if let Some(children) = self.get_children(&item.id) {
                        current_items = children;
                    } else {
                        break;
                    }
                }
            } else {
                break;
            }
        }

        selected
    }

    /// Gets the selection path.
    pub fn selection_path(&self) -> &SelectionPath {
        &self.selection
    }

    /// Returns the number of visible columns based on the current selection.
    ///
    /// This is 1 (root) + number of selected branches with loaded children.
    pub fn visible_column_count(&self) -> usize {
        let mut count = 1; // Root column is always visible

        let mut current_items: &[MillerItem<D>] = &self.roots;

        for id in &self.selection {
            if let Some(item) = current_items.iter().find(|item| &item.id == id) {
                if item.is_branch() {
                    if let Some(children) = self.get_children(&item.id) {
                        count += 1;
                        current_items = children;
                    } else {
                        // Branch is selected but children not loaded yet
                        // Still count it as it will show loading state
                        count += 1;
                        break;
                    }
                }
            } else {
                break;
            }
        }

        count
    }

    /// Gets the items for a specific column index.
    ///
    /// Column 0 is the root column.
    pub fn items_at_column(&self, column: usize) -> Option<&[MillerItem<D>]> {
        if column == 0 {
            return Some(&self.roots);
        }

        // Walk the selection path to find the parent for this column
        if column > self.selection.len() {
            return None;
        }

        let parent_id = &self.selection[column - 1];

        // Verify the parent is a branch and get its children
        let mut current_items: &[MillerItem<D>] = &self.roots;
        for (i, id) in self.selection.iter().enumerate() {
            if let Some(item) = current_items.iter().find(|item| &item.id == id) {
                if i + 1 == column {
                    // This is the parent - return its children
                    return self.get_children(parent_id);
                }
                if item.is_branch() {
                    if let Some(children) = self.get_children(&item.id) {
                        current_items = children;
                    } else {
                        return None;
                    }
                }
            } else {
                return None;
            }
        }

        None
    }

    /// Gets the column state for a specific column index.
    ///
    /// Returns `None` for the root column (always loaded).
    pub fn column_state_at(&self, column: usize) -> Option<&ColumnState<D>> {
        if column == 0 || column > self.selection.len() {
            return None;
        }

        let parent_id = &self.selection[column - 1];
        Some(self.get_column_state(parent_id))
    }

    /// Gets the selected item ID at a specific column, if any.
    pub fn selected_at(&self, column: usize) -> Option<&ItemId> {
        self.selection.get(column)
    }

    /// Sets the scroll offset for a column.
    pub fn set_scroll_offset(&mut self, column: usize, offset: f32) {
        while self.scroll_offsets.len() <= column {
            self.scroll_offsets.push(0.0);
        }
        self.scroll_offsets[column] = offset;
    }

    /// Gets the scroll offset for a column.
    pub fn scroll_offset(&self, column: usize) -> f32 {
        self.scroll_offsets.get(column).copied().unwrap_or(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_state() {
        let roots = vec![
            MillerItem::branch("1", "Item 1", ()),
            MillerItem::leaf("2", "Item 2", ()),
        ];
        let state: MillerState<()> = MillerState::new(roots);

        assert_eq!(state.roots.len(), 2);
        assert!(state.selection.is_empty());
        assert!(state.children.is_empty());
    }

    #[test]
    fn test_select_at() {
        let roots = vec![
            MillerItem::branch("1", "Item 1", ()),
            MillerItem::leaf("2", "Item 2", ()),
        ];
        let mut state: MillerState<()> = MillerState::new(roots);

        state.select_at(0, "1".to_string());
        assert_eq!(state.selection, vec!["1".to_string()]);

        state.select_at(0, "2".to_string());
        assert_eq!(state.selection, vec!["2".to_string()]);
    }

    #[test]
    fn test_visible_column_count() {
        let roots = vec![
            MillerItem::branch("1", "Item 1", ()),
            MillerItem::leaf("2", "Item 2", ()),
        ];
        let mut state: MillerState<()> = MillerState::new(roots);

        // No selection - just root column
        assert_eq!(state.visible_column_count(), 1);

        // Select a leaf - still just root (leaf doesn't add columns)
        state.select_at(0, "2".to_string());
        assert_eq!(state.visible_column_count(), 1);

        // Select a branch without children loaded
        state.select_at(0, "1".to_string());
        assert_eq!(state.visible_column_count(), 2); // Root + loading column

        // Load children for the branch
        state.set_children(
            "1".to_string(),
            vec![MillerItem::leaf("1-1", "Child 1", ())],
        );
        assert_eq!(state.visible_column_count(), 2); // Root + children column
    }
}
