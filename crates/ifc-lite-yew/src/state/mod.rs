//! Global state management for the IFC viewer
//!
//! Uses Yew's reducer pattern for predictable state updates.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::rc::Rc;
use yew::prelude::*;

// Note: HashSet doesn't implement PartialEq, so we can't derive it for ViewerState
// We implement it manually based on the fields that matter for re-rendering

/// Active tool
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Tool {
    #[default]
    Select,
    Pan,
    Orbit,
    Walk,
    Measure,
    Section,
    BoxSelect,
}

impl Tool {
    pub fn icon(&self) -> &'static str {
        match self {
            Tool::Select => "🖱️",
            Tool::Pan => "✋",
            Tool::Orbit => "🔄",
            Tool::Walk => "🚶",
            Tool::Measure => "📏",
            Tool::Section => "✂️",
            Tool::BoxSelect => "⬚",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Tool::Select => "Select (V)",
            Tool::Pan => "Pan (P)",
            Tool::Orbit => "Orbit (O)",
            Tool::Walk => "Walk (C)",
            Tool::Measure => "Measure (M)",
            Tool::Section => "Section (X)",
            Tool::BoxSelect => "Box Select (B)",
        }
    }
}

/// Theme
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Theme {
    Light,
    #[default]
    Dark,
}

/// Section plane axis
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum SectionAxis {
    X,
    #[default]
    Y,
    Z,
}

/// Section plane state
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SectionPlaneState {
    pub enabled: bool,
    pub axis: SectionAxis,
    pub position: f32, // 0.0 to 1.0
    pub flipped: bool,
}

/// Measurement point
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MeasurePoint {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Measurement between two points
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Measurement {
    pub id: u32,
    pub start: MeasurePoint,
    pub end: MeasurePoint,
}

impl Measurement {
    pub fn distance(&self) -> f32 {
        let dx = self.end.x - self.start.x;
        let dy = self.end.y - self.start.y;
        let dz = self.end.z - self.start.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// A single property value
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PropertyValue {
    pub name: String,
    pub value: String,
    pub unit: Option<String>,
}

/// A property set containing multiple properties
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PropertySet {
    pub name: String,
    pub properties: Vec<PropertyValue>,
}

/// A quantity value (length, area, volume, etc.)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QuantityValue {
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub quantity_type: String, // "Length", "Area", "Volume", "Count", "Weight", "Time"
}

/// Entity info for display
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EntityInfo {
    pub id: u64,
    pub entity_type: String,
    pub name: Option<String>,
    pub global_id: Option<String>,
    pub storey: Option<String>,
    pub storey_elevation: Option<f32>,
    pub property_sets: Vec<PropertySet>,
    pub quantities: Vec<QuantityValue>,
}

/// Storey info
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StoreyInfo {
    pub name: String,
    pub elevation: f32,
    pub entity_count: usize,
}

/// Spatial node type for hierarchy tree
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SpatialNodeType {
    Project,
    Site,
    Building,
    Storey,
    Space,
    Element,
}

/// Node in the spatial hierarchy tree
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpatialNode {
    pub id: u64,
    pub node_type: SpatialNodeType,
    pub name: String,
    pub entity_type: String,
    pub elevation: Option<f32>,
    pub children: Vec<SpatialNode>,
    pub has_geometry: bool,
}

/// Progress state
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Progress {
    pub phase: String,
    pub percent: f32,
}

/// Main viewer state
#[derive(Clone, Debug, PartialEq)]
pub struct ViewerState {
    // Loading
    pub loading: bool,
    pub progress: Option<Progress>,
    pub error: Option<String>,

    // Data
    pub entities: Vec<EntityInfo>,
    pub storeys: Vec<StoreyInfo>,
    pub spatial_tree: Option<SpatialNode>,
    pub file_name: Option<String>,

    // UI state for tree
    pub expanded_nodes: HashSet<u64>,

    // Selection
    pub selected_ids: HashSet<u64>,
    pub hovered_id: Option<u64>,

    // Visibility
    pub hidden_ids: HashSet<u64>,
    pub isolated_ids: Option<HashSet<u64>>,
    pub storey_filter: Option<String>,

    // UI
    pub active_tool: Tool,
    pub theme: Theme,
    pub left_panel_collapsed: bool,
    pub right_panel_collapsed: bool,
    pub show_shortcuts_dialog: bool,

    // Tools
    pub section_plane: SectionPlaneState,
    pub measurements: Vec<Measurement>,
    pub pending_measure_point: Option<MeasurePoint>,
    pub next_measure_id: u32,

    // Search
    pub search_query: String,
}

impl Default for ViewerState {
    fn default() -> Self {
        Self {
            loading: false,
            progress: None,
            error: None,
            entities: Vec::new(),
            storeys: Vec::new(),
            spatial_tree: None,
            file_name: None,
            expanded_nodes: HashSet::default(),
            selected_ids: HashSet::default(),
            hovered_id: None,
            hidden_ids: HashSet::default(),
            isolated_ids: None,
            storey_filter: None,
            active_tool: Tool::Select,
            theme: Theme::Dark,
            left_panel_collapsed: false,
            right_panel_collapsed: false,
            show_shortcuts_dialog: false,
            section_plane: SectionPlaneState::default(),
            measurements: Vec::new(),
            pending_measure_point: None,
            next_measure_id: 1,
            search_query: String::new(),
        }
    }
}

/// State actions
pub enum ViewerAction {
    // Loading
    SetLoading(bool),
    SetProgress(Progress),
    ClearProgress,
    SetError(String),
    ClearError,

    // Data
    SetEntities(Vec<EntityInfo>),
    SetStoreys(Vec<StoreyInfo>),
    SetSpatialTree(SpatialNode),
    SetFileName(String),
    ClearData,

    // Tree UI
    ToggleNodeExpanded(u64),
    ExpandAll,
    CollapseAll,

    // Selection
    Select(u64),
    AddToSelection(u64),
    RemoveFromSelection(u64),
    ToggleSelection(u64),
    ClearSelection,
    SetHovered(Option<u64>),

    // Visibility
    HideEntity(u64),
    ShowEntity(u64),
    ToggleVisibility(u64),
    IsolateEntity(u64),
    IsolateEntities(HashSet<u64>),
    ShowAll,
    SetStoreyFilter(Option<String>),

    // UI
    SetActiveTool(Tool),
    ToggleTheme,
    SetLeftPanelCollapsed(bool),
    SetRightPanelCollapsed(bool),
    ToggleShortcutsDialog,

    // Section plane
    SetSectionEnabled(bool),
    SetSectionAxis(SectionAxis),
    SetSectionPosition(f32),
    ToggleSectionFlip,

    // Measurements
    AddMeasurePoint(MeasurePoint),
    CompleteMeasurement,
    RemoveMeasurement(u32),
    ClearMeasurements,

    // Search
    SetSearchQuery(String),
}

impl Reducible for ViewerState {
    type Action = ViewerAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        let mut next = (*self).clone();

        match action {
            // Loading
            ViewerAction::SetLoading(loading) => {
                next.loading = loading;
            }
            ViewerAction::SetProgress(progress) => {
                next.progress = Some(progress);
            }
            ViewerAction::ClearProgress => {
                next.progress = None;
            }
            ViewerAction::SetError(error) => {
                next.error = Some(error);
                next.loading = false;
            }
            ViewerAction::ClearError => {
                next.error = None;
            }

            // Data
            ViewerAction::SetEntities(entities) => {
                next.entities = entities;
            }
            ViewerAction::SetStoreys(storeys) => {
                next.storeys = storeys;
            }
            ViewerAction::SetSpatialTree(tree) => {
                // Auto-expand root and first level
                next.expanded_nodes.insert(tree.id);
                for child in &tree.children {
                    next.expanded_nodes.insert(child.id);
                }
                next.spatial_tree = Some(tree);
            }
            ViewerAction::SetFileName(name) => {
                next.file_name = Some(name);
            }
            ViewerAction::ClearData => {
                next.entities.clear();
                next.storeys.clear();
                next.spatial_tree = None;
                next.expanded_nodes.clear();
                next.file_name = None;
                next.selected_ids.clear();
                next.hidden_ids.clear();
                next.isolated_ids = None;
                next.measurements.clear();
            }

            // Tree UI
            ViewerAction::ToggleNodeExpanded(id) => {
                if next.expanded_nodes.contains(&id) {
                    next.expanded_nodes.remove(&id);
                } else {
                    next.expanded_nodes.insert(id);
                }
            }
            ViewerAction::ExpandAll => {
                fn collect_ids(node: &SpatialNode, ids: &mut HashSet<u64>) {
                    ids.insert(node.id);
                    for child in &node.children {
                        collect_ids(child, ids);
                    }
                }
                if let Some(ref tree) = next.spatial_tree {
                    collect_ids(tree, &mut next.expanded_nodes);
                }
            }
            ViewerAction::CollapseAll => {
                next.expanded_nodes.clear();
                // Keep root expanded
                if let Some(ref tree) = next.spatial_tree {
                    next.expanded_nodes.insert(tree.id);
                }
            }

            // Selection
            ViewerAction::Select(id) => {
                next.selected_ids.clear();
                next.selected_ids.insert(id);
            }
            ViewerAction::AddToSelection(id) => {
                next.selected_ids.insert(id);
            }
            ViewerAction::RemoveFromSelection(id) => {
                next.selected_ids.remove(&id);
            }
            ViewerAction::ToggleSelection(id) => {
                if next.selected_ids.contains(&id) {
                    next.selected_ids.remove(&id);
                } else {
                    next.selected_ids.insert(id);
                }
            }
            ViewerAction::ClearSelection => {
                next.selected_ids.clear();
            }
            ViewerAction::SetHovered(id) => {
                next.hovered_id = id;
            }

            // Visibility
            ViewerAction::HideEntity(id) => {
                next.hidden_ids.insert(id);
            }
            ViewerAction::ShowEntity(id) => {
                next.hidden_ids.remove(&id);
            }
            ViewerAction::ToggleVisibility(id) => {
                if next.hidden_ids.contains(&id) {
                    next.hidden_ids.remove(&id);
                } else {
                    next.hidden_ids.insert(id);
                }
            }
            ViewerAction::IsolateEntity(id) => {
                let mut isolated = HashSet::default();
                isolated.insert(id);
                next.isolated_ids = Some(isolated);
            }
            ViewerAction::IsolateEntities(ids) => {
                next.isolated_ids = Some(ids);
            }
            ViewerAction::ShowAll => {
                next.hidden_ids.clear();
                next.isolated_ids = None;
            }
            ViewerAction::SetStoreyFilter(storey) => {
                next.storey_filter = storey;
            }

            // UI
            ViewerAction::SetActiveTool(tool) => {
                next.active_tool = tool;
            }
            ViewerAction::ToggleTheme => {
                next.theme = match next.theme {
                    Theme::Light => Theme::Dark,
                    Theme::Dark => Theme::Light,
                };
            }
            ViewerAction::SetLeftPanelCollapsed(collapsed) => {
                next.left_panel_collapsed = collapsed;
            }
            ViewerAction::SetRightPanelCollapsed(collapsed) => {
                next.right_panel_collapsed = collapsed;
            }
            ViewerAction::ToggleShortcutsDialog => {
                next.show_shortcuts_dialog = !next.show_shortcuts_dialog;
            }

            // Section plane
            ViewerAction::SetSectionEnabled(enabled) => {
                next.section_plane.enabled = enabled;
            }
            ViewerAction::SetSectionAxis(axis) => {
                next.section_plane.axis = axis;
            }
            ViewerAction::SetSectionPosition(position) => {
                next.section_plane.position = position.clamp(0.0, 1.0);
            }
            ViewerAction::ToggleSectionFlip => {
                next.section_plane.flipped = !next.section_plane.flipped;
            }

            // Measurements
            ViewerAction::AddMeasurePoint(point) => {
                if next.pending_measure_point.is_some() {
                    // Complete the measurement
                    let start = next.pending_measure_point.take().unwrap();
                    next.measurements.push(Measurement {
                        id: next.next_measure_id,
                        start,
                        end: point,
                    });
                    next.next_measure_id += 1;
                } else {
                    // Start a new measurement
                    next.pending_measure_point = Some(point);
                }
            }
            ViewerAction::CompleteMeasurement => {
                next.pending_measure_point = None;
            }
            ViewerAction::RemoveMeasurement(id) => {
                next.measurements.retain(|m| m.id != id);
            }
            ViewerAction::ClearMeasurements => {
                next.measurements.clear();
                next.pending_measure_point = None;
            }

            // Search
            ViewerAction::SetSearchQuery(query) => {
                next.search_query = query;
            }
        }

        Rc::new(next)
    }
}

/// Hook to use viewer state
#[hook]
pub fn use_viewer_state() -> UseReducerHandle<ViewerState> {
    use_reducer(ViewerState::default)
}

/// Context type for viewer state
pub type ViewerStateContext = UseReducerHandle<ViewerState>;
