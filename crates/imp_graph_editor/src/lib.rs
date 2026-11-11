//! Aura-free Imp graph-dag editor building blocks for Iced hosts.
//!
//! Provides canvas view, layout, pure edit helpers, UI prefs state, and
//! lightweight toolbar/properties widgets. Hosts supply node-type catalogs,
//! compose chrome, and map crate messages into their own shell.

mod graph_chrome;
mod graph_edit;
mod graph_layout;
mod graph_ui;
mod graph_view;

pub use graph_chrome::{
    add_type_picker, node_properties_editor, precise_ports_checkbox, tool_strip, GraphChromeMessage,
};
pub use graph_edit::{
    add_edge, add_node, auto_pair_ports, remove_edge, remove_edges_between, remove_node, rename_node,
    set_node_type, unique_edge_id, unique_node_id, CatalogPort, GraphEditError, NodeTypeCatalog,
};
pub use graph_layout::{
    layout_graph, node_size_for, GraphLayout, LaidOutNode, NODE_HEADER_HEIGHT, NODE_SIZE,
    PORT_SPACING, PRECISE_NODE_WIDTH, V_GAP,
};
pub use graph_ui::{ConnectDraft, ConnectionMode, GraphTool, GraphUiEntry, GraphUiState};
pub use graph_view::{
    center_translation, clamp_scale, graph_canvas, pan_by, zoom_at_point, GraphEdit, GraphView,
    NodePortInfo, PortCatalog, ViewState,
};
