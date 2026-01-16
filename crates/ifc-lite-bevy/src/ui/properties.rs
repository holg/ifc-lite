//! Properties panel - shows details of selected entity

use super::layout::RightPanel;
use super::styles::{UiColors, UiSizes};
use crate::{IfcSceneData, SelectionState};
use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::prelude::*;
use bevy::ui::{
    widget::Button, AlignItems, BackgroundColor, BorderColor, BorderRadius, FlexDirection,
    JustifyContent, Node, UiRect, Val,
};

pub struct PropertiesPlugin;

impl Plugin for PropertiesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_properties.after(super::layout::setup_layout))
            .add_systems(Update, update_properties);
    }
}

/// Marker for properties content
#[derive(Component)]
pub struct PropertiesContent;

/// Marker for property row
#[derive(Component)]
pub struct PropertyRow;

fn setup_properties(mut commands: Commands, panel_query: Query<Entity, With<RightPanel>>) {
    let Ok(panel_entity) = panel_query.single() else {
        return;
    };

    commands.entity(panel_entity).with_children(|panel| {
        // Panel title
        panel.spawn((
            Text::new("Properties"),
            TextFont {
                font_size: UiSizes::FONT_SIZE_LG,
                ..default()
            },
            TextColor(UiColors::TEXT_PRIMARY),
            Node {
                margin: UiRect::bottom(Val::Px(UiSizes::PADDING)),
                ..default()
            },
        ));

        // Properties content
        panel.spawn((
            PropertiesContent,
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::NONE),
        ));
    });
}

fn update_properties(
    mut commands: Commands,
    selection: Res<SelectionState>,
    scene_data: Res<IfcSceneData>,
    content_query: Query<Entity, With<PropertiesContent>>,
    existing_rows: Query<Entity, With<PropertyRow>>,
) {
    // Only update when selection changes
    if !selection.is_changed() {
        return;
    }

    let Ok(content_entity) = content_query.single() else {
        return;
    };

    // Clear existing rows - despawn() is recursive in Bevy 0.18
    for entity in existing_rows.iter() {
        commands.entity(entity).despawn();
    }

    // Get first selected entity
    let selected_id = selection.selected.iter().next();

    commands.entity(content_entity).with_children(|content| {
        if let Some(&id) = selected_id {
            // Find entity info
            if let Some(entity_info) = scene_data.entities.iter().find(|e| e.id == id) {
                // Entity type
                spawn_property_row(content, "Type", &entity_info.entity_type);

                // Name
                if let Some(ref name) = entity_info.name {
                    spawn_property_row(content, "Name", name);
                }

                // ID
                spawn_property_row(content, "ID", &format!("#{}", entity_info.id));

                // Storey
                if let Some(ref storey) = entity_info.storey {
                    spawn_property_row(content, "Storey", storey);
                }

                // Elevation
                if let Some(elevation) = entity_info.storey_elevation {
                    spawn_property_row(content, "Elevation", &format!("{:.2} m", elevation));
                }

                // Actions section
                content.spawn((
                    PropertyRow, // Mark for cleanup
                    Text::new("Actions"),
                    TextFont {
                        font_size: UiSizes::FONT_SIZE,
                        ..default()
                    },
                    TextColor(UiColors::TEXT_ACCENT),
                    Node {
                        margin: UiRect::vertical(Val::Px(UiSizes::PADDING)),
                        ..default()
                    },
                ));

                // Action buttons
                spawn_action_button(content, "Hide");
                spawn_action_button(content, "Isolate");
                spawn_action_button(content, "Focus");
            } else {
                spawn_no_selection(content);
            }
        } else {
            spawn_no_selection(content);
        }
    });
}

fn spawn_property_row(parent: &mut ChildSpawnerCommands, label: &str, value: &str) {
    parent
        .spawn((
            PropertyRow,
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::vertical(Val::Px(UiSizes::PADDING_SM)),
                border: UiRect::bottom(Val::Px(1.0)),
                ..default()
            },
            BorderColor::all(UiColors::BORDER),
        ))
        .with_children(|row: &mut ChildSpawnerCommands| {
            // Label
            row.spawn((
                Text::new(label),
                TextFont {
                    font_size: UiSizes::FONT_SIZE_SM,
                    ..default()
                },
                TextColor(UiColors::TEXT_SECONDARY),
            ));
            // Value
            row.spawn((
                Text::new(value),
                TextFont {
                    font_size: UiSizes::FONT_SIZE_SM,
                    ..default()
                },
                TextColor(UiColors::TEXT_PRIMARY),
            ));
        });
}

fn spawn_action_button(parent: &mut ChildSpawnerCommands, label: &str) {
    parent
        .spawn((
            PropertyRow, // Mark for cleanup
            Button,
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(32.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                margin: UiRect::bottom(Val::Px(UiSizes::PADDING_SM)),
                border_radius: BorderRadius::all(Val::Px(UiSizes::BORDER_RADIUS)),
                ..default()
            },
            BackgroundColor(UiColors::BUTTON_BG),
        ))
        .with_children(|btn: &mut ChildSpawnerCommands| {
            btn.spawn((
                Text::new(label),
                TextFont {
                    font_size: UiSizes::FONT_SIZE_SM,
                    ..default()
                },
                TextColor(UiColors::TEXT_PRIMARY),
            ));
        });
}

fn spawn_no_selection(parent: &mut ChildSpawnerCommands) {
    parent.spawn((
        PropertyRow, // Mark for cleanup
        Text::new("No selection"),
        TextFont {
            font_size: UiSizes::FONT_SIZE,
            ..default()
        },
        TextColor(UiColors::TEXT_SECONDARY),
        Node {
            margin: UiRect::top(Val::Px(UiSizes::PADDING * 2.0)),
            ..default()
        },
    ));

    parent.spawn((
        PropertyRow, // Mark for cleanup
        Text::new("Click on an element in the 3D view or hierarchy to see its properties."),
        TextFont {
            font_size: UiSizes::FONT_SIZE_SM,
            ..default()
        },
        TextColor(UiColors::TEXT_SECONDARY),
        Node {
            margin: UiRect::top(Val::Px(UiSizes::PADDING)),
            ..default()
        },
    ));
}
