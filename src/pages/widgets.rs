// SPDX-License-Identifier: MPL-2.0

//! Shared widgets for the Chromatic application pages.

use crate::app::{ConnectionStatus, Message};
use crate::fl;
use cosmic::prelude::*;
use cosmic::widget;

/// Connection status badge widget
pub fn connection_status_badge(status: &ConnectionStatus) -> Element<'static, Message> {
    let (text, style) = match status {
        ConnectionStatus::Disconnected => (fl!("disconnected"), cosmic::theme::Button::Standard),
        ConnectionStatus::Connecting => (fl!("connecting"), cosmic::theme::Button::Standard),
        ConnectionStatus::Connected => (fl!("connected"), cosmic::theme::Button::Suggested),
        ConnectionStatus::Error(_) => (fl!("error"), cosmic::theme::Button::Destructive),
    };

    widget::button::custom(widget::text::body(text))
        .class(style)
        .into()
}
