//! Native view embedding for iOS/macOS
//!
//! This module provides the ability to embed Bevy into a native Metal view
//! instead of creating its own window via winit.

#[cfg(target_os = "ios")]
pub mod ios;

#[cfg(target_os = "macos")]
pub mod macos;

mod app_views;
mod plugin;

pub use app_views::AppViews;
pub use plugin::AppViewPlugin;

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::ops::Deref;

/// Wrapper to make raw pointers Send + Sync (unsafe, user must ensure thread safety)
#[derive(Clone, Copy)]
pub struct SendSyncWrapper<T>(pub T);

unsafe impl<T> Send for SendSyncWrapper<T> {}
unsafe impl<T> Sync for SendSyncWrapper<T> {}

impl<T> SendSyncWrapper<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }
}

impl<T> Deref for SendSyncWrapper<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// View object for iOS (CAMetalLayer backing a UIView)
#[cfg(target_os = "ios")]
pub struct IOSViewObj {
    pub view: *mut std::ffi::c_void,
    pub scale_factor: f32,
}

/// View object for macOS (CAMetalLayer backing an NSView)
#[cfg(target_os = "macos")]
pub struct MacOSViewObj {
    pub view: *mut std::ffi::c_void,
    pub scale_factor: f32,
}

/// Unified AppView that wraps platform-specific view objects
pub struct AppView {
    #[cfg(target_os = "ios")]
    inner: SendSyncWrapper<IOSViewObj>,
    #[cfg(target_os = "macos")]
    inner: SendSyncWrapper<MacOSViewObj>,
    #[cfg(not(any(target_os = "ios", target_os = "macos")))]
    inner: (),
}

impl AppView {
    #[cfg(target_os = "ios")]
    pub fn new(obj: IOSViewObj) -> Self {
        Self {
            inner: SendSyncWrapper::new(obj),
        }
    }

    #[cfg(target_os = "macos")]
    pub fn new(obj: MacOSViewObj) -> Self {
        Self {
            inner: SendSyncWrapper::new(obj),
        }
    }

    #[cfg(not(any(target_os = "ios", target_os = "macos")))]
    pub fn new() -> Self {
        Self { inner: () }
    }

    /// Get the scale factor for this view
    pub fn scale_factor(&self) -> f32 {
        #[cfg(target_os = "ios")]
        {
            self.inner.scale_factor
        }
        #[cfg(target_os = "macos")]
        {
            self.inner.scale_factor
        }
        #[cfg(not(any(target_os = "ios", target_os = "macos")))]
        {
            1.0
        }
    }

    /// Get the logical resolution of the view
    pub fn logical_resolution(&self) -> (f32, f32) {
        #[cfg(target_os = "ios")]
        {
            ios::get_view_size(self.inner.view)
        }
        #[cfg(target_os = "macos")]
        {
            macos::get_view_size(self.inner.view)
        }
        #[cfg(not(any(target_os = "ios", target_os = "macos")))]
        {
            (800.0, 600.0)
        }
    }
}

#[cfg(target_os = "ios")]
impl HasWindowHandle for AppView {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        ios::get_window_handle(&self.inner)
    }
}

#[cfg(target_os = "ios")]
impl HasDisplayHandle for AppView {
    fn display_handle(
        &self,
    ) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        ios::get_display_handle()
    }
}

#[cfg(target_os = "macos")]
impl HasWindowHandle for AppView {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        macos::get_window_handle(&self.inner)
    }
}

#[cfg(target_os = "macos")]
impl HasDisplayHandle for AppView {
    fn display_handle(
        &self,
    ) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        macos::get_display_handle()
    }
}
