//! Keeps the app's window out of screenshots and screen recordings when the
//! school blocks them for its students (engine [`kg_engine::Restriction`]).
//!
//! Windows only for now: the window shows black or is left out of captures
//! (`SetWindowDisplayAffinity`). macOS and Linux do not block yet; the
//! settings say so.
#![cfg_attr(windows, allow(unsafe_code))]

/// Applies the school's choice to the main window when it changes.
#[derive(Debug, Default)]
pub(crate) struct CaptureGuard {
    applied: Option<bool>,
}

impl CaptureGuard {
    pub(crate) fn sync(&mut self, frame: &eframe::Frame, block: bool) {
        if self.applied == Some(block) {
            return;
        }
        // Tried once per change, so a failure does not repeat every frame.
        self.applied = Some(block);
        if let Err(err) = set_blocked(frame, block) {
            tracing::warn!(%err, block, "could not change screen capture of the window");
        }
    }
}

/// Whether this system can keep the window out of captures.
pub(crate) const SUPPORTED: bool = cfg!(windows);

#[cfg(windows)]
fn set_blocked(frame: &eframe::Frame, block: bool) -> Result<(), String> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE, WDA_MONITOR, WDA_NONE,
    };

    let handle = frame.window_handle().map_err(|e| e.to_string())?;
    let RawWindowHandle::Win32(window) = handle.as_raw() else {
        return Err("not a Win32 window".into());
    };
    let hwnd = window.hwnd.get() as windows_sys::Win32::Foundation::HWND;
    let affinity = if block {
        WDA_EXCLUDEFROMCAPTURE
    } else {
        WDA_NONE
    };
    // SAFETY: `hwnd` is this process's live top-level window, from winit.
    if unsafe { SetWindowDisplayAffinity(hwnd, affinity) } != 0 {
        return Ok(());
    }
    // Windows before 10 (2004) only knows "black in captures".
    // SAFETY: as above.
    if block && unsafe { SetWindowDisplayAffinity(hwnd, WDA_MONITOR) } != 0 {
        return Ok(());
    }
    Err(std::io::Error::last_os_error().to_string())
}

#[cfg(not(windows))]
fn set_blocked(_frame: &eframe::Frame, block: bool) -> Result<(), String> {
    if block {
        Err("this system cannot keep the window out of captures yet".into())
    } else {
        Ok(())
    }
}
