//! Main UI layout - toolbar, panels, viewport

use super::styles::{UiColors, UiSizes};
use bevy::prelude::*;
use bevy::ui::{
    AlignItems, BackgroundColor, FlexDirection, Node, Overflow, ScrollPosition, UiRect, Val,
};

pub struct LayoutPlugin;

impl Plugin for LayoutPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_ui_camera, setup_layout).chain());
    }
}

/// Marker for the UI camera
#[derive(Component)]
pub struct UiOnlyCamera;

/// Setup a dedicated 2D camera for UI rendering
fn setup_ui_camera(mut commands: Commands) {
    println!("[UI] Setting up UI camera...");
    commands.spawn((
        Camera2d,
        Camera {
            // Render UI on top of 3D (higher order = rendered later)
            order: 1,
            // Don't clear - preserve 3D render underneath
            clear_color: bevy::prelude::ClearColorConfig::None,
            ..default()
        },
        UiOnlyCamera,
    ));
}

/// Marker for the root UI node
#[derive(Component)]
pub struct UiRoot;

/// Marker for the toolbar container
#[derive(Component)]
pub struct ToolbarContainer;

/// Marker for the left panel (hierarchy)
#[derive(Component)]
pub struct LeftPanel;

/// Marker for the right panel (properties)
#[derive(Component)]
pub struct RightPanel;

/// Marker for the viewport area (where 3D renders)
#[derive(Component)]
pub struct ViewportArea;

/// Marker for the status bar
#[derive(Component)]
pub struct StatusBar;

pub fn setup_layout(mut commands: Commands) {
    println!("[UI] Setting up layout...");

    // Root container - full screen flexbox (transparent to show 3D behind)
    commands
        .spawn((
            UiRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::NONE),
        ))
        .with_children(|parent| {
            // Top toolbar
            parent.spawn((
                ToolbarContainer,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(UiSizes::TOOLBAR_HEIGHT),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    padding: UiRect::horizontal(Val::Px(UiSizes::PADDING)),
                    ..default()
                },
                BackgroundColor(UiColors::TOOLBAR_BG),
            ));

            // Main content area (panels + viewport)
            parent
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        flex_direction: FlexDirection::Row,
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                ))
                .with_children(|content| {
                    // Left panel (hierarchy) - scrollable
                    content.spawn((
                        LeftPanel,
                        super::ScrollablePanel,
                        Node {
                            width: Val::Px(UiSizes::PANEL_WIDTH),
                            height: Val::Percent(100.0),
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(UiSizes::PADDING)),
                            overflow: Overflow::scroll_y(),
                            ..default()
                        },
                        BackgroundColor(UiColors::PANEL_BG),
                        Interaction::default(),
                        ScrollPosition::default(),
                    ));

                    // Viewport spacer (3D renders behind this)
                    content.spawn((
                        ViewportArea,
                        Node {
                            flex_grow: 1.0,
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        BackgroundColor(Color::NONE),
                    ));

                    // Right panel (properties) - scrollable
                    content.spawn((
                        RightPanel,
                        super::ScrollablePanel,
                        Node {
                            width: Val::Px(UiSizes::PANEL_WIDTH),
                            height: Val::Percent(100.0),
                            flex_direction: FlexDirection::Column,
                            padding: UiRect::all(Val::Px(UiSizes::PADDING)),
                            overflow: Overflow::scroll_y(),
                            ..default()
                        },
                        BackgroundColor(UiColors::PANEL_BG),
                        Interaction::default(),
                        ScrollPosition::default(),
                    ));
                });

            // Bottom status bar
            parent.spawn((
                StatusBar,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(24.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    padding: UiRect::horizontal(Val::Px(UiSizes::PADDING)),
                    ..default()
                },
                BackgroundColor(UiColors::TOOLBAR_BG),
            ));
        });
}
