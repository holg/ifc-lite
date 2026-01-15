//! UI styling constants and helpers

use bevy::prelude::*;
use bevy::ui::{AlignItems, JustifyContent, UiRect, Val};

/// Color palette for the UI
pub struct UiColors;

impl UiColors {
    // Background colors
    pub const PANEL_BG: Color = Color::srgba(0.15, 0.15, 0.15, 0.95);
    pub const TOOLBAR_BG: Color = Color::srgba(0.12, 0.12, 0.12, 0.98);
    pub const BUTTON_BG: Color = Color::srgba(0.25, 0.25, 0.25, 1.0);
    pub const BUTTON_HOVER: Color = Color::srgba(0.35, 0.35, 0.35, 1.0);
    pub const BUTTON_ACTIVE: Color = Color::srgba(0.2, 0.5, 0.8, 1.0);

    // Text colors
    pub const TEXT_PRIMARY: Color = Color::srgba(0.9, 0.9, 0.9, 1.0);
    pub const TEXT_SECONDARY: Color = Color::srgba(0.6, 0.6, 0.6, 1.0);
    pub const TEXT_ACCENT: Color = Color::srgba(0.4, 0.7, 1.0, 1.0);

    // Border colors
    pub const BORDER: Color = Color::srgba(0.3, 0.3, 0.3, 1.0);
    pub const BORDER_HOVER: Color = Color::srgba(0.5, 0.5, 0.5, 1.0);

    // Selection colors
    pub const SELECTED: Color = Color::srgba(0.2, 0.5, 0.8, 0.3);
    pub const HOVER: Color = Color::srgba(0.4, 0.4, 0.4, 0.3);
}

/// Common sizes
pub struct UiSizes;

impl UiSizes {
    pub const TOOLBAR_HEIGHT: f32 = 48.0;
    pub const PANEL_WIDTH: f32 = 280.0;
    pub const BUTTON_SIZE: f32 = 36.0;
    pub const ICON_SIZE: f32 = 20.0;
    pub const PADDING: f32 = 8.0;
    pub const PADDING_SM: f32 = 4.0;
    pub const BORDER_RADIUS: f32 = 4.0;
    pub const FONT_SIZE: f32 = 14.0;
    pub const FONT_SIZE_SM: f32 = 12.0;
    pub const FONT_SIZE_LG: f32 = 16.0;
}

/// Create a standard panel style
pub fn panel_style() -> Node {
    Node {
        padding: UiRect::all(Val::Px(UiSizes::PADDING)),
        ..default()
    }
}

/// Create a standard button style
pub fn button_style() -> Node {
    Node {
        width: Val::Px(UiSizes::BUTTON_SIZE),
        height: Val::Px(UiSizes::BUTTON_SIZE),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        margin: UiRect::all(Val::Px(UiSizes::PADDING_SM)),
        ..default()
    }
}

/// Create a text button style (variable width)
pub fn text_button_style() -> Node {
    Node {
        height: Val::Px(UiSizes::BUTTON_SIZE),
        padding: UiRect::horizontal(Val::Px(UiSizes::PADDING * 2.0)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        margin: UiRect::all(Val::Px(UiSizes::PADDING_SM)),
        ..default()
    }
}
