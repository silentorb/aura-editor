//! Lightweight toolbar / properties widget building blocks.
//!
//! Hosts compose these into a full chrome story (panes, commands, menus).

use iced::widget::{button, checkbox, column, pick_list, row, text, text_input};
use iced::{Element, Length};

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

/// Select / Add / Connect / Erase tool buttons.
pub fn tool_strip(active: GraphTool) -> Element<'static, GraphChromeMessage> {
    let select_label = if active == GraphTool::Select {
        "[Select]"
    } else {
        "Select"
    };
    let add_label = if active == GraphTool::Add {
        "[Add]"
    } else {
        "Add"
    };
    let connect_label = if active == GraphTool::Connect {
        "[Connect]"
    } else {
        "Connect"
    };
    let erase_label = if active == GraphTool::Erase {
        "[Erase]"
    } else {
        "Erase"
    };

    row![
        button(select_label).on_press(GraphChromeMessage::SetTool(GraphTool::Select)),
        button(add_label).on_press(GraphChromeMessage::SetTool(GraphTool::Add)),
        button(connect_label).on_press(GraphChromeMessage::SetTool(GraphTool::Connect)),
        button(erase_label).on_press(GraphChromeMessage::SetTool(GraphTool::Erase)),
    ]
    .spacing(6)
    .into()
}

/// Node-type picker for the Add tool.
pub fn add_type_picker(
    type_ids: Vec<String>,
    selected: String,
) -> Element<'static, GraphChromeMessage> {
    pick_list(type_ids, Some(selected), GraphChromeMessage::SetAddNodeType).into()
}

/// Precise-ports checkbox (hosts may wrap this in a menu).
pub fn precise_ports_checkbox(precise: bool) -> Element<'static, GraphChromeMessage> {
    checkbox(precise)
        .label("Precise ports")
        .on_toggle(|enabled| {
            GraphChromeMessage::SetConnectionMode(if enabled {
                ConnectionMode::Precise
            } else {
                ConnectionMode::Simple
            })
        })
        .width(Length::Fill)
        .into()
}

/// Node id / type editor for a selected node.
pub fn node_properties_editor(
    node_id: &str,
    rename_buffer: &str,
    type_ids: Vec<String>,
    selected_type: Option<String>,
) -> Element<'static, GraphChromeMessage> {
    column![
        text(format!("Node: {node_id}")).size(14),
        text("Id:"),
        text_input("node id", rename_buffer)
            .on_input(GraphChromeMessage::RenameInput)
            .on_submit(GraphChromeMessage::RenameCommit)
            .width(Length::Fill),
        button("Rename").on_press(GraphChromeMessage::RenameCommit),
        text("Type:"),
        pick_list(type_ids, selected_type, GraphChromeMessage::SelectedTypeChanged),
    ]
    .spacing(8)
    .into()
}
