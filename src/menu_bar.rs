//! Global application menu bar (File / Edit / View / Play).
//!
//! Menus are shell-level chrome only. Per-view controls (for example the
//! piano-roll tool strip) stay on toolbars.

use iced::widget::{button, text};
use iced::{Element, Length};
use iced_aw::menu::{Menu, MenuBar};
use iced_aw::menu_items;

use crate::{
    CommandId, Message, CMD_DOCUMENT_CLOSE_ACTIVE, CMD_DOCUMENT_NEW_GRAPH, CMD_DOCUMENT_NEW_NOTATION,
    CMD_DOCUMENT_OPEN, CMD_DOCUMENT_SAVE, CMD_PLAY_START, CMD_PLAY_STOP,
};

fn menu_item<'a>(label: &'a str, command: &'static str) -> iced::widget::Button<'a, Message> {
    button(text(label))
        .on_press(Message::Command(CommandId::new(command)))
        .width(Length::Fill)
}

/// Builds the global File / Edit / View / Play menu bar.
pub fn global_menu_bar<'a>() -> Element<'a, Message> {
    let file_menu = Menu::new(menu_items!(
        (menu_item("Open…", CMD_DOCUMENT_OPEN)),
        (menu_item("New notation", CMD_DOCUMENT_NEW_NOTATION)),
        (menu_item("New graph", CMD_DOCUMENT_NEW_GRAPH)),
        (menu_item("Save", CMD_DOCUMENT_SAVE)),
        (menu_item("Close", CMD_DOCUMENT_CLOSE_ACTIVE)),
    ))
    .max_width(220.0);

    let play_menu = Menu::new(menu_items!(
        (menu_item("Start", CMD_PLAY_START)),
        (menu_item("Stop", CMD_PLAY_STOP)),
    ))
    .max_width(180.0);

    // Edit / View stay as labeled roots until they have items. iced_aw panics on
    // empty Menu overlays (indexes into zero children), so do not attach empty menus.
    MenuBar::new(menu_items!(
        (button(text("File")), file_menu),
        (button(text("Edit"))),
        (button(text("View"))),
        (button(text("Play")), play_menu),
    ))
    .padding([4, 8])
    .spacing(4)
    .close_on_item_click(true)
    .close_on_background_click(true)
    .into()
}
