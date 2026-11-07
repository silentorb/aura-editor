//! Global application menu bar (File / Edit / View / Play).
//!
//! Menus are shell-level chrome only. Per-view controls (for example the
//! piano-roll tool strip) stay on toolbars.

use iced::widget::{button, text};
use iced::{Element, Length};
use iced_aw::menu::{Menu, MenuBar};
use iced_aw::menu_items;
use native_theme_iced::{
    at_least, button_content_min_size, button_padding, font_size, styles,
    AccessibilityPreferences, ResolvedTheme,
};

use crate::{
    CommandId, Message, CMD_DOCUMENT_CLOSE_ACTIVE, CMD_DOCUMENT_NEW_GRAPH, CMD_DOCUMENT_NEW_NOTATION,
    CMD_DOCUMENT_OPEN, CMD_DOCUMENT_SAVE, CMD_PLAY_START, CMD_PLAY_STOP,
};

fn menu_item<'a>(
    label: &'a str,
    command: &'static str,
    resolved: &'a ResolvedTheme,
    body_size: f32,
) -> iced::widget::Button<'a, Message> {
    button(at_least(
        text(label).size(body_size),
        button_content_min_size(resolved),
    ))
    .on_press(Message::Command(CommandId::new(command)))
    .padding(button_padding(resolved))
    .style(styles::button(resolved))
    .width(Length::Fill)
}

fn menu_root<'a>(
    label: &'a str,
    resolved: &'a ResolvedTheme,
    body_size: f32,
    on_press: Option<Message>,
) -> iced::widget::Button<'a, Message> {
    button(at_least(
        text(label).size(body_size),
        button_content_min_size(resolved),
    ))
    .on_press_maybe(on_press)
    .padding(button_padding(resolved))
    .style(styles::button(resolved))
}

/// Builds the global File / Edit / View / Play menu bar.
pub fn global_menu_bar<'a>(
    resolved: &'a ResolvedTheme,
    accessibility: &'a AccessibilityPreferences,
) -> Element<'a, Message> {
    let body = font_size(resolved, accessibility);

    let file_menu = Menu::new(menu_items!(
        (menu_item("Open…", CMD_DOCUMENT_OPEN, resolved, body)),
        (menu_item(
            "New notation",
            CMD_DOCUMENT_NEW_NOTATION,
            resolved,
            body
        )),
        (menu_item("New graph", CMD_DOCUMENT_NEW_GRAPH, resolved, body)),
        (menu_item("Save", CMD_DOCUMENT_SAVE, resolved, body)),
        (menu_item(
            "Close",
            CMD_DOCUMENT_CLOSE_ACTIVE,
            resolved,
            body
        )),
    ))
    .max_width(220.0);

    let play_menu = Menu::new(menu_items!(
        (menu_item("Start", CMD_PLAY_START, resolved, body)),
        (menu_item("Stop", CMD_PLAY_STOP, resolved, body)),
    ))
    .max_width(180.0);

    // Edit / View stay as labeled roots until they have items. iced_aw panics on
    // empty Menu overlays (indexes into zero children), so do not attach empty menus.
    // Interactive roots need a no-op on_press so iced does not paint them Disabled.
    MenuBar::new(menu_items!(
        (
            menu_root("File", resolved, body, Some(Message::MenuRoot)),
            file_menu
        ),
        (menu_root("Edit", resolved, body, None)),
        (menu_root("View", resolved, body, None)),
        (
            menu_root("Play", resolved, body, Some(Message::MenuRoot)),
            play_menu
        ),
    ))
    .padding([4, 8])
    .spacing(4)
    .close_on_item_click(true)
    .close_on_background_click(true)
    .style(styles::aw::menu(resolved))
    .into()
}
