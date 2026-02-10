// SPDX-License-Identifier: MPL-2.0

//! Shared widgets for the Chromatic application pages.

use crate::app::{ConnectionStatus, Message, Notification, NotificationLevel};
use crate::fl;
use cosmic::iced::{Alignment, Length};
use cosmic::prelude::*;
use cosmic::widget::{self, icon};

/// Connection status badge widget
pub fn connection_status_badge(status: &ConnectionStatus) -> Element<'static, Message> {
    match status {
        ConnectionStatus::Disconnected => {
            widget::button::custom(widget::text::body(fl!("disconnected")))
                .class(cosmic::theme::Button::Standard)
                .into()
        }
        ConnectionStatus::Connecting => {
            widget::button::custom(widget::text::body(fl!("connecting")))
                .class(cosmic::theme::Button::Standard)
                .into()
        }
        ConnectionStatus::Connected => {
            // Green checkmark for connected status
            widget::button::custom(
                widget::row::with_capacity(2)
                    .push(icon::from_name("object-select-symbolic").size(16))
                    .push(widget::text::body(fl!("connected")))
                    .spacing(4)
                    .align_y(Alignment::Center),
            )
            .class(cosmic::theme::Button::Suggested)
            .into()
        }
        ConnectionStatus::Error(_) => widget::button::custom(widget::text::body(fl!("error")))
            .class(cosmic::theme::Button::Destructive)
            .into(),
    }
}

/// Notification toast widget
pub fn notification_toast(notification: &Notification) -> Element<'_, Message> {
    let id = notification.id;

    // Choose style based on level
    let container_style = match notification.level {
        NotificationLevel::Info => cosmic::style::Container::Card,
        NotificationLevel::Success => cosmic::style::Container::Card,
        NotificationLevel::Warning => cosmic::style::Container::Card,
        NotificationLevel::Error => cosmic::style::Container::Card,
    };

    // Icon based on level
    let level_icon = match notification.level {
        NotificationLevel::Info => icon::from_name("dialog-information-symbolic").size(20),
        NotificationLevel::Success => icon::from_name("object-select-symbolic").size(20),
        NotificationLevel::Warning => icon::from_name("dialog-warning-symbolic").size(20),
        NotificationLevel::Error => icon::from_name("dialog-error-symbolic").size(20),
    };

    // Content row with icon, text, and buttons
    let content = widget::row::with_capacity(4)
        .push(level_icon)
        .push(
            widget::column::with_capacity(2)
                .push(widget::text::body(&notification.title))
                .push_maybe(if notification.message.is_empty() {
                    None
                } else {
                    Some(widget::text::caption(&notification.message))
                })
                .spacing(2)
                .width(Length::Fill),
        )
        .push(
            widget::button::icon(icon::from_name("edit-copy-symbolic"))
                .on_press(Message::CopyNotification(id))
                .class(cosmic::theme::Button::Standard),
        )
        .push(
            widget::button::icon(icon::from_name("window-close-symbolic"))
                .on_press(Message::DismissNotification(id))
                .class(cosmic::theme::Button::Standard),
        )
        .spacing(8)
        .align_y(Alignment::Center);

    widget::container(content)
        .padding(12)
        .width(Length::Fixed(400.0))
        .class(container_style)
        .into()
}
