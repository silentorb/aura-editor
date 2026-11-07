//! Lightweight toolbar / properties widget building blocks.
//!
//! Hosts compose these into a full chrome story (panes, commands, menus).

use iced::widget::{button, checkbox, column, pick_list, row, text, text_input};
use iced::{Element, Length, Renderer};
use native_theme_iced::{
    at_least, button_content_min_size, button_padding, combo_box_padding, control_line_height,
    font_size, input_padding, pick_list_handle, scaled_text_size, styles, AccessibilityPreferences,
    ResolvedTheme,
};

use crate::graph_ui::{ConnectionMode, GraphTool};

/// Messages emitted by graph chrome building blocks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphChromeMessage {
    /// Activate a modal tool.
    SetTool(GraphTool),
    /// Choose the Add-tool node type.
    SetAddNodeType(String),
    /// Toggle simple vs precise connection UI.
    SetConnectionMode(ConnectionMode),
    /// Rename buffer edited.
    RenameInput(String),
    /// Commit the rename buffer.
    RenameCommit,
    /// Change the selected node's type.
    SelectedTypeChanged(String),
}

fn tool_button<'a>(
    label: &'a str,
    active: bool,
    message: GraphChromeMessage,
    resolved: &ResolvedTheme,
) -> iced::widget::Button<'a, GraphChromeMessage> {
    button(at_least(text(label), button_content_min_size(resolved)))
        .on_press(message)
        .padding(button_padding(resolved))
        .style(styles::button_toggle(resolved, active))
}

/// Select / Add / Connect / Erase tool buttons.
pub fn tool_strip<'a>(
    active: GraphTool,
    resolved: &'a ResolvedTheme,
) -> Element<'a, GraphChromeMessage> {
    row![
        tool_button(
            "Select",
            active == GraphTool::Select,
            GraphChromeMessage::SetTool(GraphTool::Select),
            resolved,
        ),
        tool_button(
            "Add",
            active == GraphTool::Add,
            GraphChromeMessage::SetTool(GraphTool::Add),
            resolved,
        ),
        tool_button(
            "Connect",
            active == GraphTool::Connect,
            GraphChromeMessage::SetTool(GraphTool::Connect),
            resolved,
        ),
        tool_button(
            "Erase",
            active == GraphTool::Erase,
            GraphChromeMessage::SetTool(GraphTool::Erase),
            resolved,
        ),
    ]
    .spacing(6)
    .into()
}

/// Node-type picker for the Add tool.
pub fn add_type_picker<'a>(
    type_ids: Vec<String>,
    selected: String,
    resolved: &'a ResolvedTheme,
    accessibility: &'a AccessibilityPreferences,
) -> Element<'a, GraphChromeMessage> {
    let size = scaled_text_size(resolved.combo_box.font.size, accessibility);
    let padding = combo_box_padding(resolved);
    pick_list(type_ids, Some(selected), GraphChromeMessage::SetAddNodeType)
        .text_size(size)
        .padding(padding)
        .text_line_height(control_line_height(
            resolved,
            size,
            resolved.combo_box.min_height,
            padding,
        ))
        .handle(pick_list_handle::<Renderer>(resolved))
        .style(styles::pick_list(resolved))
        .menu_style(styles::menu(resolved))
        .into()
}

/// Precise-ports checkbox (hosts may wrap this in a menu).
pub fn precise_ports_checkbox<'a>(
    precise: bool,
    resolved: &'a ResolvedTheme,
) -> Element<'a, GraphChromeMessage> {
    checkbox(precise)
        .label("Precise ports")
        .on_toggle(|enabled| {
            GraphChromeMessage::SetConnectionMode(if enabled {
                ConnectionMode::Precise
            } else {
                ConnectionMode::Simple
            })
        })
        .size(resolved.checkbox.indicator_width)
        .spacing(resolved.checkbox.label_gap)
        .style(styles::checkbox(resolved))
        .width(Length::Fill)
        .into()
}

/// Node id / type editor for a selected node.
pub fn node_properties_editor<'a>(
    node_id: &str,
    rename_buffer: &str,
    type_ids: Vec<String>,
    selected_type: Option<String>,
    resolved: &'a ResolvedTheme,
    accessibility: &'a AccessibilityPreferences,
) -> Element<'a, GraphChromeMessage> {
    let body = font_size(resolved, accessibility);
    let input_size = scaled_text_size(resolved.input.font.size, accessibility);
    let input_pad = input_padding(resolved);
    let combo_size = scaled_text_size(resolved.combo_box.font.size, accessibility);
    let combo_pad = combo_box_padding(resolved);

    column![
        text(format!("Node: {node_id}")).size(body),
        text("Id:").size(body),
        text_input("node id", rename_buffer)
            .on_input(GraphChromeMessage::RenameInput)
            .on_submit(GraphChromeMessage::RenameCommit)
            .size(input_size)
            .padding(input_pad)
            .line_height(control_line_height(
                resolved,
                input_size,
                resolved.input.min_height,
                input_pad,
            ))
            .style(styles::text_input(resolved))
            .width(Length::Fill),
        button(at_least(text("Rename"), button_content_min_size(resolved)))
            .on_press(GraphChromeMessage::RenameCommit)
            .padding(button_padding(resolved))
            .style(styles::button_primary(resolved)),
        text("Type:").size(body),
        pick_list(type_ids, selected_type, GraphChromeMessage::SelectedTypeChanged)
            .text_size(combo_size)
            .padding(combo_pad)
            .text_line_height(control_line_height(
                resolved,
                combo_size,
                resolved.combo_box.min_height,
                combo_pad,
            ))
            .handle(pick_list_handle::<Renderer>(resolved))
            .style(styles::pick_list(resolved))
            .menu_style(styles::menu(resolved)),
    ]
    .spacing(8)
    .into()
}
