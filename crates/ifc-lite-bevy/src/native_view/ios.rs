//! iOS-specific view handling
//!
//! Provides raw window handle implementation for UIView-backed CAMetalLayer.

use super::{IOSViewObj, SendSyncWrapper};
use core_graphics::geometry::CGRect;
use raw_window_handle::{
    DisplayHandle, HandleError, RawDisplayHandle, RawWindowHandle, UiKitDisplayHandle,
    UiKitWindowHandle, WindowHandle,
};

/// Get the view size from the UIView
pub fn get_view_size(view: *mut std::ffi::c_void) -> (f32, f32) {
    #[cfg(target_os = "ios")]
    {
        use objc::runtime::Object;
        use objc::*;

        unsafe {
            let view = view as *mut Object;
            // Get frame: CGRect
            let frame: CGRect = msg_send![view, frame];
            (frame.size.width as f32, frame.size.height as f32)
        }
    }
    #[cfg(not(target_os = "ios"))]
    {
        let _ = view;
        (800.0, 600.0)
    }
}

/// Get window handle for iOS UIView
pub fn get_window_handle(
    view_obj: &SendSyncWrapper<IOSViewObj>,
) -> Result<WindowHandle<'_>, HandleError> {
    let mut handle =
        UiKitWindowHandle::new(std::ptr::NonNull::new(view_obj.view as *mut _).unwrap());

    let raw = RawWindowHandle::UiKit(handle);
    // SAFETY: The view pointer is valid for the lifetime of the AppView
    Ok(unsafe { WindowHandle::borrow_raw(raw) })
}

/// Get display handle for iOS
pub fn get_display_handle() -> Result<DisplayHandle<'static>, HandleError> {
    let handle = UiKitDisplayHandle::new();
    let raw = RawDisplayHandle::UiKit(handle);
    // SAFETY: iOS display handle doesn't require any specific pointer
    Ok(unsafe { DisplayHandle::borrow_raw(raw) })
}
