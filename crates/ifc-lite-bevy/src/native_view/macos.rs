//! macOS-specific view handling
//!
//! Provides raw window handle implementation for NSView-backed CAMetalLayer.

use super::{MacOSViewObj, SendSyncWrapper};
use core_graphics::geometry::CGRect;
use raw_window_handle::{DisplayHandle, HandleError, RawDisplayHandle, RawWindowHandle, WindowHandle, AppKitDisplayHandle, AppKitWindowHandle};

/// Get the view size from the NSView
pub fn get_view_size(view: *mut std::ffi::c_void) -> (f32, f32) {
    #[cfg(target_os = "macos")]
    {
        use objc::runtime::Object;
        use objc::*;

        unsafe {
            let view = view as *mut Object;
            // Get frame: NSRect (same as CGRect)
            let frame: CGRect = msg_send![view, frame];
            (frame.size.width as f32, frame.size.height as f32)
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = view;
        (800.0, 600.0)
    }
}

/// Get window handle for macOS NSView
pub fn get_window_handle(view_obj: &SendSyncWrapper<MacOSViewObj>) -> Result<WindowHandle<'_>, HandleError> {
    let handle = AppKitWindowHandle::new(std::ptr::NonNull::new(view_obj.view as *mut _).unwrap());

    let raw = RawWindowHandle::AppKit(handle);
    // SAFETY: The view pointer is valid for the lifetime of the AppView
    Ok(unsafe { WindowHandle::borrow_raw(raw) })
}

/// Get display handle for macOS
pub fn get_display_handle() -> Result<DisplayHandle<'static>, HandleError> {
    let handle = AppKitDisplayHandle::new();
    let raw = RawDisplayHandle::AppKit(handle);
    // SAFETY: macOS display handle doesn't require any specific pointer
    Ok(unsafe { DisplayHandle::borrow_raw(raw) })
}
