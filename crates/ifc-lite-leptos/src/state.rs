//! Global state management for the IFC viewer using Leptos signals
//!
//! Uses fine-grained signals for reactive updates.

use crate::bridge::ColorPalette;
use leptos::prelude::*;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};

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
    pub description: Option<String>,
    pub global_id: Option<String>,
    pub storey: Option<String>,
    pub storey_elevation: Option<f32>,
    pub property_sets: Vec<PropertySet>,
    pub quantities: Vec<QuantityValue>,
}

impl EntityInfo {
    /// Get display label: prefer description, then name, then type#id
    pub fn display_label(&self) -> String {
        if let Some(ref desc) = self.description {
            if !desc.is_empty() && desc != "$" {
                return desc.clone();
            }
        }
        if let Some(ref name) = self.name {
            if !name.is_empty() && name != "$" {
                return name.clone();
            }
        }
        format!("#{}", self.id)
    }
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

// ============================================================================
// Leptos Signal-based State Groups
// ============================================================================

/// Loading state signals
#[derive(Clone, Copy)]
pub struct LoadingState {
    pub loading: RwSignal<bool>,
    pub progress: RwSignal<Option<Progress>>,
    pub error: RwSignal<Option<String>>,
}

impl LoadingState {
    pub fn new() -> Self {
        Self {
            loading: RwSignal::new(false),
            progress: RwSignal::new(None),
            error: RwSignal::new(None),
        }
    }

    pub fn set_loading(&self, loading: bool) {
        self.loading.set(loading);
    }

    pub fn set_progress(&self, progress: Progress) {
        self.progress.set(Some(progress));
    }

    pub fn clear_progress(&self) {
        self.progress.set(None);
    }

    pub fn set_error(&self, error: String) {
        self.error.set(Some(error));
        self.loading.set(false);
    }

    pub fn clear_error(&self) {
        self.error.set(None);
    }
}

impl Default for LoadingState {
    fn default() -> Self {
        Self::new()
    }
}

/// Selection state signals
#[derive(Clone, Copy)]
pub struct SelectionState {
    pub selected_ids: RwSignal<FxHashSet<u64>>,
    pub hovered_id: RwSignal<Option<u64>>,
}

impl SelectionState {
    pub fn new() -> Self {
        Self {
            selected_ids: RwSignal::new(FxHashSet::default()),
            hovered_id: RwSignal::new(None),
        }
    }

    pub fn select(&self, id: u64) {
        self.selected_ids.update(|ids| {
            ids.clear();
            ids.insert(id);
        });
    }

    pub fn add_to_selection(&self, id: u64) {
        self.selected_ids.update(|ids| {
            ids.insert(id);
        });
    }

    pub fn remove_from_selection(&self, id: u64) {
        self.selected_ids.update(|ids| {
            ids.remove(&id);
        });
    }

    pub fn toggle_selection(&self, id: u64) {
        self.selected_ids.update(|ids| {
            if !ids.remove(&id) {
                ids.insert(id);
            }
        });
    }

    pub fn clear(&self) {
        self.selected_ids.set(FxHashSet::default());
    }

    pub fn set_hovered(&self, id: Option<u64>) {
        self.hovered_id.set(id);
    }
}

impl Default for SelectionState {
    fn default() -> Self {
        Self::new()
    }
}

/// Visibility state signals
#[derive(Clone, Copy)]
pub struct VisibilityState {
    pub hidden_ids: RwSignal<FxHashSet<u64>>,
    pub isolated_ids: RwSignal<Option<FxHashSet<u64>>>,
    pub storey_filter: RwSignal<Option<String>>,
}

impl VisibilityState {
    pub fn new() -> Self {
        Self {
            hidden_ids: RwSignal::new(FxHashSet::default()),
            isolated_ids: RwSignal::new(None),
            storey_filter: RwSignal::new(None),
        }
    }

    pub fn hide(&self, id: u64) {
        self.hidden_ids.update(|ids| {
            ids.insert(id);
        });
    }

    pub fn show(&self, id: u64) {
        self.hidden_ids.update(|ids| {
            ids.remove(&id);
        });
    }

    pub fn toggle_visibility(&self, id: u64) {
        self.hidden_ids.update(|ids| {
            if !ids.remove(&id) {
                ids.insert(id);
            }
        });
    }

    pub fn isolate(&self, id: u64) {
        let mut isolated = FxHashSet::default();
        isolated.insert(id);
        self.isolated_ids.set(Some(isolated));
    }

    pub fn isolate_many(&self, ids: FxHashSet<u64>) {
        self.isolated_ids.set(Some(ids));
    }

    pub fn show_all(&self) {
        self.hidden_ids.set(FxHashSet::default());
        self.isolated_ids.set(None);
    }

    pub fn set_storey_filter(&self, storey: Option<String>) {
        self.storey_filter.set(storey);
    }
}

impl Default for VisibilityState {
    fn default() -> Self {
        Self::new()
    }
}

/// UI state signals
#[derive(Clone, Copy)]
pub struct UiState {
    pub active_tool: RwSignal<Tool>,
    pub theme: RwSignal<Theme>,
    pub left_panel_collapsed: RwSignal<bool>,
    pub right_panel_collapsed: RwSignal<bool>,
    pub show_shortcuts_dialog: RwSignal<bool>,
    pub search_query: RwSignal<String>,
    pub color_palette: RwSignal<ColorPalette>,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            active_tool: RwSignal::new(Tool::Select),
            theme: RwSignal::new(Theme::Dark),
            left_panel_collapsed: RwSignal::new(false),
            right_panel_collapsed: RwSignal::new(false),
            show_shortcuts_dialog: RwSignal::new(false),
            search_query: RwSignal::new(String::new()),
            color_palette: RwSignal::new(ColorPalette::default()),
        }
    }

    pub fn set_tool(&self, tool: Tool) {
        self.active_tool.set(tool);
    }

    pub fn toggle_theme(&self) {
        self.theme.update(|t| {
            *t = match t {
                Theme::Light => Theme::Dark,
                Theme::Dark => Theme::Light,
            };
        });
    }

    pub fn toggle_left_panel(&self) {
        self.left_panel_collapsed.update(|c| *c = !*c);
    }

    pub fn toggle_right_panel(&self) {
        self.right_panel_collapsed.update(|c| *c = !*c);
    }

    pub fn toggle_shortcuts_dialog(&self) {
        self.show_shortcuts_dialog.update(|s| *s = !*s);
    }

    pub fn set_search(&self, query: String) {
        self.search_query.set(query);
    }

    pub fn cycle_palette(&self) {
        self.color_palette.update(|p| *p = p.next());
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self::new()
    }
}

/// Scene data signals
#[derive(Clone, Copy)]
pub struct SceneState {
    pub entities: RwSignal<Vec<EntityInfo>>,
    pub storeys: RwSignal<Vec<StoreyInfo>>,
    pub spatial_tree: RwSignal<Option<SpatialNode>>,
    pub file_name: RwSignal<Option<String>>,
    pub expanded_nodes: RwSignal<FxHashSet<u64>>,
}

impl SceneState {
    pub fn new() -> Self {
        Self {
            entities: RwSignal::new(Vec::new()),
            storeys: RwSignal::new(Vec::new()),
            spatial_tree: RwSignal::new(None),
            file_name: RwSignal::new(None),
            expanded_nodes: RwSignal::new(FxHashSet::default()),
        }
    }

    pub fn set_entities(&self, entities: Vec<EntityInfo>) {
        self.entities.set(entities);
    }

    pub fn set_storeys(&self, storeys: Vec<StoreyInfo>) {
        self.storeys.set(storeys);
    }

    pub fn set_spatial_tree(&self, tree: SpatialNode) {
        // Auto-expand root and first level
        self.expanded_nodes.update(|exp| {
            exp.insert(tree.id);
            for child in &tree.children {
                exp.insert(child.id);
            }
        });
        self.spatial_tree.set(Some(tree));
    }

    pub fn set_file_name(&self, name: String) {
        self.file_name.set(Some(name));
    }

    pub fn toggle_node_expanded(&self, id: u64) {
        self.expanded_nodes.update(|exp| {
            if !exp.remove(&id) {
                exp.insert(id);
            }
        });
    }

    pub fn expand_all(&self) {
        fn collect_ids(node: &SpatialNode, ids: &mut FxHashSet<u64>) {
            ids.insert(node.id);
            for child in &node.children {
                collect_ids(child, ids);
            }
        }

        if let Some(tree) = self.spatial_tree.get_untracked() {
            self.expanded_nodes.update(|exp| {
                collect_ids(&tree, exp);
            });
        }
    }

    pub fn collapse_all(&self) {
        self.expanded_nodes.update(|exp| {
            exp.clear();
            // Keep root expanded
            if let Some(tree) = self.spatial_tree.get_untracked() {
                exp.insert(tree.id);
            }
        });
    }

    pub fn clear(&self) {
        self.entities.set(Vec::new());
        self.storeys.set(Vec::new());
        self.spatial_tree.set(None);
        self.file_name.set(None);
        self.expanded_nodes.set(FxHashSet::default());
    }
}

impl Default for SceneState {
    fn default() -> Self {
        Self::new()
    }
}

/// Section plane signals
#[derive(Clone, Copy)]
pub struct SectionState {
    pub enabled: RwSignal<bool>,
    pub axis: RwSignal<SectionAxis>,
    pub position: RwSignal<f32>,
    pub flipped: RwSignal<bool>,
}

impl SectionState {
    pub fn new() -> Self {
        Self {
            enabled: RwSignal::new(false),
            axis: RwSignal::new(SectionAxis::Y),
            position: RwSignal::new(0.5),
            flipped: RwSignal::new(false),
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.set(enabled);
    }

    pub fn set_axis(&self, axis: SectionAxis) {
        self.axis.set(axis);
    }

    pub fn set_position(&self, position: f32) {
        self.position.set(position.clamp(0.0, 1.0));
    }

    pub fn toggle_flip(&self) {
        self.flipped.update(|f| *f = !*f);
    }
}

impl Default for SectionState {
    fn default() -> Self {
        Self::new()
    }
}

/// Measurement state signals
#[derive(Clone, Copy)]
pub struct MeasurementState {
    pub measurements: RwSignal<Vec<Measurement>>,
    pub pending_point: RwSignal<Option<MeasurePoint>>,
    pub next_id: RwSignal<u32>,
}

impl MeasurementState {
    pub fn new() -> Self {
        Self {
            measurements: RwSignal::new(Vec::new()),
            pending_point: RwSignal::new(None),
            next_id: RwSignal::new(1),
        }
    }

    pub fn add_point(&self, point: MeasurePoint) {
        if let Some(start) = self.pending_point.get_untracked() {
            // Complete measurement
            let id = self.next_id.get_untracked();
            self.measurements.update(|m| {
                m.push(Measurement {
                    id,
                    start,
                    end: point,
                });
            });
            self.next_id.update(|n| *n += 1);
            self.pending_point.set(None);
        } else {
            // Start new measurement
            self.pending_point.set(Some(point));
        }
    }

    pub fn cancel_pending(&self) {
        self.pending_point.set(None);
    }

    pub fn remove(&self, id: u32) {
        self.measurements.update(|m| m.retain(|meas| meas.id != id));
    }

    pub fn clear(&self) {
        self.measurements.set(Vec::new());
        self.pending_point.set(None);
    }
}

impl Default for MeasurementState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Combined Viewer State
// ============================================================================

/// Combined viewer state with all signal groups
#[derive(Clone, Copy)]
pub struct ViewerState {
    pub loading: LoadingState,
    pub selection: SelectionState,
    pub visibility: VisibilityState,
    pub ui: UiState,
    pub scene: SceneState,
    pub section: SectionState,
    pub measurements: MeasurementState,
}

impl ViewerState {
    pub fn new() -> Self {
        Self {
            loading: LoadingState::new(),
            selection: SelectionState::new(),
            visibility: VisibilityState::new(),
            ui: UiState::new(),
            scene: SceneState::new(),
            section: SectionState::new(),
            measurements: MeasurementState::new(),
        }
    }
}

impl Default for ViewerState {
    fn default() -> Self {
        Self::new()
    }
}

/// Provide viewer state to the component tree
pub fn provide_viewer_state() {
    let state = ViewerState::new();
    provide_context(state);
}

/// Hook to access viewer state from context
pub fn use_viewer_state() -> ViewerState {
    expect_context::<ViewerState>()
}
