//! Toolbar UI component

use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::prelude::*;
use bevy::ui::{
    widget::Button, AlignItems, BackgroundColor, BorderRadius, Interaction, JustifyContent, Node,
    UiRect, Val,
};

use super::layout::ToolbarContainer;
use super::styles::{UiColors, UiSizes};

pub struct ToolbarPlugin;

impl Plugin for ToolbarPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_toolbar.after(super::layout::setup_layout))
            .add_systems(Update, button_interaction);
    }
}

/// Marker for toolbar buttons
#[derive(Component)]
pub struct ToolbarButton {
    pub action: ButtonAction,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonAction {
    OpenFile,
    Home,
    FitAll,
    ToggleHierarchy,
    ToggleProperties,
    ToggleSection,
}

fn setup_toolbar(mut commands: Commands, toolbar_query: Query<Entity, With<ToolbarContainer>>) {
    let Ok(toolbar_entity) = toolbar_query.single() else {
        return;
    };

    commands.entity(toolbar_entity).with_children(|toolbar| {
        // File section
        spawn_button(toolbar, "Open", ButtonAction::OpenFile);
        spawn_separator(toolbar);

        // View section
        spawn_button(toolbar, "Home", ButtonAction::Home);
        spawn_button(toolbar, "Fit", ButtonAction::FitAll);
        spawn_separator(toolbar);

        // Panel toggles
        spawn_button(toolbar, "Tree", ButtonAction::ToggleHierarchy);
        spawn_button(toolbar, "Props", ButtonAction::ToggleProperties);
        spawn_separator(toolbar);

        // Tools
        spawn_button(toolbar, "Section", ButtonAction::ToggleSection);

        // Spacer
        toolbar.spawn(Node {
            flex_grow: 1.0,
            ..default()
        });

        // Right side - title/status
        toolbar.spawn((
            Text::new("IFC-Lite Viewer"),
            TextFont {
                font_size: UiSizes::FONT_SIZE,
                ..default()
            },
            TextColor(UiColors::TEXT_SECONDARY),
        ));
    });
}

fn spawn_button(parent: &mut ChildSpawnerCommands, label: &str, action: ButtonAction) {
    parent
        .spawn((
            ToolbarButton { action },
            Button,
            Node {
                height: Val::Px(UiSizes::BUTTON_SIZE),
                padding: UiRect::horizontal(Val::Px(12.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                margin: UiRect::horizontal(Val::Px(2.0)),
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

fn spawn_separator(parent: &mut ChildSpawnerCommands) {
    parent.spawn((
        Node {
            width: Val::Px(1.0),
            height: Val::Px(24.0),
            margin: UiRect::horizontal(Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(UiColors::BORDER),
    ));
}

fn button_interaction(
    mut query: Query<(&Interaction, &mut BackgroundColor, &ToolbarButton), Changed<Interaction>>,
    mut ui_state: ResMut<super::UiState>,
    mut left_panel: Query<
        &mut Visibility,
        (
            With<super::layout::LeftPanel>,
            Without<super::layout::RightPanel>,
        ),
    >,
    mut right_panel: Query<
        &mut Visibility,
        (
            With<super::layout::RightPanel>,
            Without<super::layout::LeftPanel>,
        ),
    >,
    mut open_dialog_events: MessageWriter<crate::loader::OpenFileDialogRequest>,
    mut camera_controller: ResMut<crate::camera::CameraController>,
    scene_data: Res<crate::IfcSceneData>,
) {
    for (interaction, mut bg_color, button) in query.iter_mut() {
        match *interaction {
            Interaction::Pressed => {
                *bg_color = BackgroundColor(UiColors::BUTTON_ACTIVE);

                // Handle action directly
                match button.action {
                    ButtonAction::ToggleHierarchy => {
                        ui_state.show_hierarchy = !ui_state.show_hierarchy;
                        if let Ok(mut vis) = left_panel.single_mut() {
                            *vis = if ui_state.show_hierarchy {
                                Visibility::Inherited
                            } else {
                                Visibility::Hidden
                            };
                        }
                    }
                    ButtonAction::ToggleProperties => {
                        ui_state.show_properties = !ui_state.show_properties;
                        if let Ok(mut vis) = right_panel.single_mut() {
                            *vis = if ui_state.show_properties {
                                Visibility::Inherited
                            } else {
                                Visibility::Hidden
                            };
                        }
                    }
                    ButtonAction::OpenFile => {
                        crate::log_info("[UI] Requesting file dialog...");
                        open_dialog_events.write(crate::loader::OpenFileDialogRequest);
                    }
                    ButtonAction::Home => {
                        crate::log("[UI] Home requested");
                        // Reset camera to default isometric view
                        camera_controller.azimuth = 0.785; // 45 degrees
                        camera_controller.elevation = 0.615; // ~35 degrees
                    }
                    ButtonAction::FitAll => {
                        crate::log("[UI] Fit all requested");
                        // Fit camera to scene bounds
                        if let Some(ref bounds) = scene_data.bounds {
                            camera_controller.frame(bounds.min, bounds.max);
                        }
                    }
                    ButtonAction::ToggleSection => {
                        crate::log("[UI] Toggle section requested");
                    }
                }
            }
            Interaction::Hovered => {
                *bg_color = BackgroundColor(UiColors::BUTTON_HOVER);
            }
            Interaction::None => {
                *bg_color = BackgroundColor(UiColors::BUTTON_BG);
            }
        }
    }
}
