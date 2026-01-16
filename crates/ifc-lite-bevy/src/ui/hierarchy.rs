//! Hierarchy panel - tree view of IFC entities by storey

use super::layout::LeftPanel;
use super::styles::{UiColors, UiSizes};
use crate::{EntityInfo, IfcSceneData, SelectionState};
use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::prelude::*;
use bevy::ui::{
    widget::Button, AlignItems, BackgroundColor, BorderRadius, FlexDirection, Interaction, Node,
    Overflow, UiRect, Val,
};

pub struct HierarchyPlugin;

impl Plugin for HierarchyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_hierarchy.after(super::layout::setup_layout))
            .add_systems(Update, (update_hierarchy, handle_entity_click));
    }
}

/// Marker for hierarchy panel content
#[derive(Component)]
pub struct HierarchyContent;

/// Marker for search input
#[derive(Component)]
pub struct SearchInput;

/// Marker for entity list item
#[derive(Component)]
pub struct EntityListItem {
    pub entity_id: u64,
}

fn setup_hierarchy(mut commands: Commands, panel_query: Query<Entity, With<LeftPanel>>) {
    let Ok(panel_entity) = panel_query.single() else {
        return;
    };

    commands.entity(panel_entity).with_children(|panel| {
        // Panel title
        panel.spawn((
            Text::new("Model Hierarchy"),
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

        // Search bar placeholder
        panel
            .spawn((
                SearchInput,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Px(32.0),
                    margin: UiRect::bottom(Val::Px(UiSizes::PADDING)),
                    padding: UiRect::horizontal(Val::Px(UiSizes::PADDING)),
                    align_items: AlignItems::Center,
                    border_radius: BorderRadius::all(Val::Px(UiSizes::BORDER_RADIUS)),
                    ..default()
                },
                BackgroundColor(UiColors::BUTTON_BG),
            ))
            .with_children(|search: &mut ChildSpawnerCommands| {
                search.spawn((
                    Text::new("Search..."),
                    TextFont {
                        font_size: UiSizes::FONT_SIZE_SM,
                        ..default()
                    },
                    TextColor(UiColors::TEXT_SECONDARY),
                ));
            });

        // Scrollable entity list
        panel.spawn((
            HierarchyContent,
            Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                flex_direction: FlexDirection::Column,
                overflow: Overflow::scroll_y(),
                ..default()
            },
            BackgroundColor(Color::NONE),
            Interaction::default(),
            bevy::ui::ScrollPosition::default(),
        ));
    });
}

/// Marker for hierarchy group (type category)
#[derive(Component)]
pub struct HierarchyGroup {
    pub type_name: String,
    pub expanded: bool,
}

/// Marker for hierarchy item (cleanup marker)
#[derive(Component)]
pub struct HierarchyItem;

fn update_hierarchy(
    mut commands: Commands,
    scene_data: Res<IfcSceneData>,
    content_query: Query<Entity, With<HierarchyContent>>,
    existing_items: Query<Entity, With<HierarchyItem>>,
) {
    // Only update when scene data changes
    if !scene_data.is_changed() {
        return;
    }

    let Ok(content_entity) = content_query.single() else {
        return;
    };

    // Clear existing items - despawn() is recursive in Bevy 0.18
    for entity in existing_items.iter() {
        commands.entity(entity).despawn();
    }

    // Group entities by type (e.g., IfcWall, IfcWindow, etc.)
    let mut type_groups: std::collections::BTreeMap<String, Vec<&EntityInfo>> =
        std::collections::BTreeMap::new();

    for entity in &scene_data.entities {
        // Use clean type name without "Ifc" prefix for display
        let type_key = entity
            .entity_type
            .strip_prefix("Ifc")
            .unwrap_or(&entity.entity_type)
            .to_string();
        type_groups.entry(type_key).or_default().push(entity);
    }

    // Build hierarchy grouped by type
    commands.entity(content_entity).with_children(|content| {
        for (type_name, entities) in type_groups {
            // Type group header (collapsible)
            content
                .spawn((
                    HierarchyItem,
                    HierarchyGroup {
                        type_name: type_name.clone(),
                        expanded: true, // Start expanded
                    },
                    Button,
                    Node {
                        width: Val::Percent(100.0),
                        padding: UiRect::all(Val::Px(UiSizes::PADDING_SM)),
                        margin: UiRect::top(Val::Px(UiSizes::PADDING_SM)),
                        border_radius: BorderRadius::all(Val::Px(UiSizes::BORDER_RADIUS)),
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(UiColors::BUTTON_BG),
                ))
                .with_children(|header: &mut ChildSpawnerCommands| {
                    // Expand/collapse indicator
                    header.spawn((
                        Text::new("▼ "),
                        TextFont {
                            font_size: UiSizes::FONT_SIZE_SM,
                            ..default()
                        },
                        TextColor(UiColors::TEXT_SECONDARY),
                    ));
                    // Type name with count
                    header.spawn((
                        Text::new(format!("{} ({})", type_name, entities.len())),
                        TextFont {
                            font_size: UiSizes::FONT_SIZE_SM,
                            ..default()
                        },
                        TextColor(UiColors::TEXT_ACCENT),
                    ));
                });

            // Entity items under this type
            for entity_info in entities {
                let display_name = entity_info
                    .name
                    .clone()
                    .unwrap_or_else(|| format!("#{}", entity_info.id));

                content
                    .spawn((
                        HierarchyItem,
                        EntityListItem {
                            entity_id: entity_info.id,
                        },
                        Button,
                        Node {
                            width: Val::Percent(100.0),
                            padding: UiRect::new(
                                Val::Px(UiSizes::PADDING * 2.0), // Indented
                                Val::Px(UiSizes::PADDING),
                                Val::Px(UiSizes::PADDING_SM),
                                Val::Px(UiSizes::PADDING_SM),
                            ),
                            ..default()
                        },
                        BackgroundColor(Color::NONE),
                    ))
                    .with_children(|item: &mut ChildSpawnerCommands| {
                        // Entity name only (type is in parent group)
                        item.spawn((
                            Text::new(display_name),
                            TextFont {
                                font_size: UiSizes::FONT_SIZE_SM,
                                ..default()
                            },
                            TextColor(UiColors::TEXT_PRIMARY),
                        ));
                    });
            }
        }
    });
}

fn handle_entity_click(
    mut query: Query<(&Interaction, &EntityListItem, &mut BackgroundColor), Changed<Interaction>>,
    mut selection: ResMut<SelectionState>,
) {
    for (interaction, item, mut bg_color) in query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                selection.select(item.entity_id);
                *bg_color = BackgroundColor(UiColors::SELECTED);
            }
            Interaction::Hovered => {
                *bg_color = BackgroundColor(UiColors::HOVER);
            }
            Interaction::None => {
                if selection.is_selected(item.entity_id) {
                    *bg_color = BackgroundColor(UiColors::SELECTED);
                } else {
                    *bg_color = BackgroundColor(Color::NONE);
                }
            }
        }
    }
}
