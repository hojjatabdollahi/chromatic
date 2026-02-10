// SPDX-License-Identifier: MPL-2.0

//! Miller columns widget builder and rendering.

use super::item::{MillerItem, MillerItemType};
use super::message::MillerMessage;
use super::state::{ColumnState, MillerState, SelectionPath};
use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::widget::{self, icon};
use std::fmt::Debug;

/// Builder for the Miller columns widget.
///
/// # Type Parameters
///
/// - `D`: The custom data type associated with each item
/// - `Message`: The parent's message type
///
/// # Example
///
/// ```ignore
/// MillerColumns::new(&miller_state, Message::Miller)
///     .column_width(Length::Fixed(200.0))
///     .min_column_width(150)
///     .max_columns(4)
///     .into()
/// ```
pub struct MillerColumns<'a, D, Message>
where
    D: Clone + Debug + 'a,
    Message: Clone + 'static,
{
    state: &'a MillerState<D>,
    on_message: Box<dyn Fn(MillerMessage<D>) -> Message + 'a>,
    column_width: Length,
    column_height: Length,
    #[allow(dead_code)]
    min_column_width: u16,
    max_columns: Option<usize>,
    spacing: u16,
    item_view: Option<Box<dyn Fn(&MillerItem<D>, bool) -> Element<'a, Message> + 'a>>,
    loading_view: Option<Box<dyn Fn() -> Element<'a, Message> + 'a>>,
    empty_view: Option<Box<dyn Fn() -> Element<'a, Message> + 'a>>,
    error_view: Option<Box<dyn Fn(&str) -> Element<'a, Message> + 'a>>,
}

impl<'a, D, Message> MillerColumns<'a, D, Message>
where
    D: Clone + Debug + 'a,
    Message: Clone + 'static,
{
    /// Creates a new Miller columns widget.
    ///
    /// # Arguments
    ///
    /// - `state`: The Miller state (owned by parent)
    /// - `on_message`: Function to wrap `MillerMessage` into the parent's `Message` type
    pub fn new(
        state: &'a MillerState<D>,
        on_message: impl Fn(MillerMessage<D>) -> Message + 'a,
    ) -> Self {
        Self {
            state,
            on_message: Box::new(on_message),
            column_width: Length::Fixed(200.0),
            column_height: Length::Fill,
            min_column_width: 150,
            max_columns: None,
            spacing: 4,
            item_view: None,
            loading_view: None,
            empty_view: None,
            error_view: None,
        }
    }

    /// Sets the column width strategy.
    ///
    /// Default is `Length::Fixed(200.0)`.
    pub fn column_width(mut self, width: Length) -> Self {
        self.column_width = width;
        self
    }

    /// Sets the column height strategy.
    ///
    /// Default is `Length::Fill`. Use `Length::Fixed(...)` or `Length::FillPortion(...)`
    /// when placing inside a horizontal scrollable.
    pub fn column_height(mut self, height: Length) -> Self {
        self.column_height = height;
        self
    }

    /// Sets the minimum column width in pixels.
    ///
    /// Default is 150.
    pub fn min_column_width(mut self, width: u16) -> Self {
        self.min_column_width = width;
        self
    }

    /// Limits the maximum number of visible columns.
    ///
    /// When the selection path is longer than this, earlier columns
    /// will scroll out of view.
    pub fn max_columns(mut self, max: usize) -> Self {
        self.max_columns = Some(max);
        self
    }

    /// Sets the spacing between columns.
    ///
    /// Default is 4.
    pub fn spacing(mut self, spacing: u16) -> Self {
        self.spacing = spacing;
        self
    }

    /// Sets a custom item renderer.
    ///
    /// The function receives the item and whether it's selected,
    /// and should return an `Element` to display.
    pub fn item_view<F>(mut self, renderer: F) -> Self
    where
        F: Fn(&MillerItem<D>, bool) -> Element<'a, Message> + 'a,
    {
        self.item_view = Some(Box::new(renderer));
        self
    }

    /// Sets a custom loading indicator view.
    pub fn loading_view<F>(mut self, renderer: F) -> Self
    where
        F: Fn() -> Element<'a, Message> + 'a,
    {
        self.loading_view = Some(Box::new(renderer));
        self
    }

    /// Sets a custom empty column view.
    pub fn empty_view<F>(mut self, renderer: F) -> Self
    where
        F: Fn() -> Element<'a, Message> + 'a,
    {
        self.empty_view = Some(Box::new(renderer));
        self
    }

    /// Sets a custom error view.
    pub fn error_view<F>(mut self, renderer: F) -> Self
    where
        F: Fn(&str) -> Element<'a, Message> + 'a,
    {
        self.error_view = Some(Box::new(renderer));
        self
    }

    /// Renders a single item using the default renderer.
    fn default_item_view(item: &MillerItem<D>, is_selected: bool) -> Element<'a, Message> {
        let icon_name = match item.item_type {
            MillerItemType::Branch => "go-next-symbolic",
            MillerItemType::Leaf => "emblem-documents-symbolic",
        };

        // Clone the label to own it
        let label = item.label.clone();

        let row = widget::row::with_capacity(2)
            .push(widget::text::body(label).width(Length::Fill))
            .push(icon::from_name(icon_name).size(16))
            .align_y(Alignment::Center)
            .spacing(8);

        let container_class = if is_selected {
            cosmic::style::Container::Primary
        } else {
            cosmic::style::Container::default()
        };

        widget::container(row)
            .padding(8)
            .width(Length::Fill)
            .class(container_class)
            .into()
    }

    /// Renders the default loading view.
    fn default_loading_view() -> Element<'a, Message> {
        widget::container(widget::text::body("Loading..."))
            .padding(16)
            .width(Length::Fill)
            .align_x(cosmic::iced::alignment::Horizontal::Center)
            .into()
    }

    /// Renders the default empty view.
    fn default_empty_view() -> Element<'a, Message> {
        widget::container(widget::text::caption("No items"))
            .padding(16)
            .width(Length::Fill)
            .align_x(cosmic::iced::alignment::Horizontal::Center)
            .into()
    }

    /// Renders the default error view.
    fn default_error_view(error: &str) -> Element<'a, Message> {
        // Clone the error string to own it
        let error_text = error.to_string();

        widget::container(
            widget::column::with_capacity(2)
                .push(icon::from_name("dialog-error-symbolic").size(24))
                .push(widget::text::caption(error_text))
                .spacing(8)
                .align_x(Alignment::Center),
        )
        .padding(16)
        .width(Length::Fill)
        .align_x(cosmic::iced::alignment::Horizontal::Center)
        .into()
    }

    /// Renders a single item.
    fn render_item(&self, item: &MillerItem<D>, is_selected: bool) -> Element<'a, Message> {
        if let Some(ref renderer) = self.item_view {
            renderer(item, is_selected)
        } else {
            Self::default_item_view(item, is_selected)
        }
    }

    /// Renders the loading state.
    fn render_loading(&self) -> Element<'a, Message> {
        if let Some(ref renderer) = self.loading_view {
            renderer()
        } else {
            Self::default_loading_view()
        }
    }

    /// Renders the empty state.
    fn render_empty(&self) -> Element<'a, Message> {
        if let Some(ref renderer) = self.empty_view {
            renderer()
        } else {
            Self::default_empty_view()
        }
    }

    /// Renders the error state.
    fn render_error(&self, error: &str) -> Element<'a, Message> {
        if let Some(ref renderer) = self.error_view {
            renderer(error)
        } else {
            Self::default_error_view(error)
        }
    }

    /// Renders a single column with items.
    fn render_column(
        &self,
        column_index: usize,
        items: &[MillerItem<D>],
        current_path: SelectionPath,
    ) -> Element<'a, Message> {
        let selected_id = self.state.selected_at(column_index);

        if items.is_empty() {
            return self.render_empty();
        }

        let mut column = widget::column::with_capacity(items.len()).spacing(2);

        for item in items {
            let is_selected = selected_id.map_or(false, |id| id == &item.id);
            let item_clone = item.clone();
            let item_for_activate = item.clone();

            // Build the path to this item
            let mut item_path = current_path.clone();
            item_path.push(item.id.clone());

            let item_path_for_activate = item_path.clone();

            let on_message = &self.on_message;

            // Create message for selection
            let select_msg = on_message(MillerMessage::Select {
                column: column_index,
                path: item_path.clone(),
                item: item_clone.clone(),
            });

            // Wrap item in mouse_area for click handling
            let item_element = self.render_item(item, is_selected);

            let clickable = if item.is_leaf() {
                // For leaf items, single click selects, we could add double-click for activate
                // but for now single click also activates
                let activate_msg = on_message(MillerMessage::Activate {
                    path: item_path_for_activate,
                    item: item_for_activate,
                });
                widget::mouse_area(item_element)
                    .on_press(select_msg)
                    .on_release(activate_msg)
            } else {
                // For branch items, click selects and triggers child loading
                widget::mouse_area(item_element).on_press(select_msg)
            };

            column = column.push(clickable);
        }

        widget::scrollable(column)
            .width(self.column_width)
            .height(self.column_height)
            .into()
    }

    /// Renders a column in loading state.
    fn render_loading_column(&self) -> Element<'a, Message> {
        widget::container(self.render_loading())
            .width(self.column_width)
            .height(self.column_height)
            .into()
    }

    /// Renders a column in error state.
    fn render_error_column(&self, error: &str) -> Element<'a, Message> {
        widget::container(self.render_error(error))
            .width(self.column_width)
            .height(self.column_height)
            .into()
    }

    /// Builds the widget and returns it as an Element.
    pub fn build(self) -> Element<'a, Message> {
        let visible_count = self.state.visible_column_count();
        let start_column = if let Some(max) = self.max_columns {
            if visible_count > max {
                visible_count - max
            } else {
                0
            }
        } else {
            0
        };

        let mut row = widget::row::with_capacity(visible_count).spacing(self.spacing);

        // Track the path as we traverse
        let mut current_path: SelectionPath = Vec::new();

        for col in start_column..visible_count {
            if col == 0 {
                // Root column
                let column_element = self.render_column(0, &self.state.roots, current_path.clone());
                row = row.push(
                    widget::container(column_element)
                        .class(cosmic::style::Container::Card)
                        .height(self.column_height),
                );
            } else {
                // Child column - get the parent ID from selection
                let parent_id = &self.state.selection[col - 1];
                current_path.push(parent_id.clone());

                let column_state = self.state.get_column_state(parent_id);

                let column_element: Element<'a, Message> = match column_state {
                    ColumnState::NotLoaded => {
                        // Emit NeedChildren message via a placeholder
                        // In practice, this should be handled in update() when selection changes
                        self.render_loading_column()
                    }
                    ColumnState::Loading => self.render_loading_column(),
                    ColumnState::Loaded(children) => {
                        self.render_column(col, children, current_path.clone())
                    }
                    ColumnState::Error(error) => self.render_error_column(error),
                };

                row = row.push(
                    widget::container(column_element)
                        .class(cosmic::style::Container::Card)
                        .height(self.column_height),
                );
            }
        }

        // Wrap in a container that allows horizontal overflow
        widget::container(row)
            .width(Length::Shrink)
            .height(self.column_height)
            .into()
    }
}

impl<'a, D, Message> From<MillerColumns<'a, D, Message>> for Element<'a, Message>
where
    D: Clone + Debug + 'a,
    Message: Clone + 'static,
{
    fn from(miller: MillerColumns<'a, D, Message>) -> Self {
        miller.build()
    }
}
