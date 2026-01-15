//! App views manager
//!
//! Manages the mapping between Bevy entities and native views.

use super::AppView;
use bevy::prelude::*;
use rustc_hash::FxHashMap;
use uuid::Uuid;

/// Resource that manages native views
#[derive(Default)]
pub struct AppViews {
    /// Map from window ID to view
    views: FxHashMap<Entity, AppViewWindow>,
    /// Map from entity to window ID
    entity_to_window: FxHashMap<Entity, Entity>,
}

/// Wrapper around AppView for use in Bevy
pub struct AppViewWindow {
    pub view: AppView,
    pub id: Uuid,
}

impl AppViews {
    /// Create a new AppViews manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a window from a native view object
    #[cfg(target_os = "ios")]
    pub fn create_window(&mut self, view_obj: super::IOSViewObj, entity: Entity) -> &AppViewWindow {
        let view = AppView::new(view_obj);
        let id = Uuid::new_v4();

        let window = AppViewWindow { view, id };
        self.views.insert(entity, window);
        self.entity_to_window.insert(entity, entity);

        self.views.get(&entity).unwrap()
    }

    /// Create a window from a native view object
    #[cfg(target_os = "macos")]
    pub fn create_window(&mut self, view_obj: super::MacOSViewObj, entity: Entity) -> &AppViewWindow {
        let view = AppView::new(view_obj);
        let id = Uuid::new_v4();

        let window = AppViewWindow { view, id };
        self.views.insert(entity, window);
        self.entity_to_window.insert(entity, entity);

        self.views.get(&entity).unwrap()
    }

    /// Get a view by entity
    pub fn get_view(&self, entity: Entity) -> Option<&AppViewWindow> {
        self.entity_to_window
            .get(&entity)
            .and_then(|e| self.views.get(e))
    }

    /// Remove a view
    pub fn remove_view(&mut self, entity: Entity) -> Option<AppViewWindow> {
        if let Some(window_entity) = self.entity_to_window.remove(&entity) {
            self.views.remove(&window_entity)
        } else {
            None
        }
    }

    /// Check if there are any views
    pub fn has_views(&self) -> bool {
        !self.views.is_empty()
    }

    /// Get the first view (for single-view apps)
    pub fn first_view(&self) -> Option<&AppViewWindow> {
        self.views.values().next()
    }
}
