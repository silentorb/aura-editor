//! Global application menu bar (File / Edit / View).
//!
//! Menus are shell-level chrome only. Per-view controls (for example the
//! piano-roll tool strip) stay on toolbars.

use iced::widget::{button, text};
use iced::{Element, Length};
use iced_aw::menu::{Menu, MenuBar};
use iced_aw::menu_items;

use crate::{
    CommandId, Message, CMD_DOCUMENT_CLOSE_ACTIVE, CMD_DOCUMENT_NEW_NOTATION, CMD_DOCUMENT_OPEN,
    CMD_DOCUMENT_SAVE,
};

fn file_item<'a>(label: &'a str, command: &'static str) -> iced::widget::Button<'a, Message> {
    button(text(label))
        .on_press(Message::Command(CommandId::new(command)))
        .width(Length::Fill)
}

/// Builds the global File / Edit / View menu bar.
pub fn global_menu_bar<'a>() -> Element<'a, Message> {
    let file_menu = Menu::new(menu_items!(
        (file_item("Open…", CMD_DOCUMENT_OPEN)),
        (file_item("New notation", CMD_DOCUMENT_NEW_NOTATION)),
        (file_item("Save", CMD_DOCUMENT_SAVE)),
        (file_item("Close", CMD_DOCUMENT_CLOSE_ACTIVE)),
    ))
    .max_width(220.0);

    // Edit / View stay as labeled roots until they have items. iced_aw panics on
    // empty Menu overlays (indexes into zero children), so do not attach empty menus.
    MenuBar::new(menu_items!(
        (button(text("File")), file_menu),
        (button(text("Edit"))),
        (button(text("View"))),
    ))
    .padding([4, 8])
    .spacing(4)
    .close_on_item_click(true)
    .close_on_background_click(true)
    .into()
}
