//! The brand's icon on the taskbar. eframe sets only the window's small icon
//! when it changes, and Windows' taskbar shows the big one, which otherwise
//! stays the executable's own: a school's icon from the panel would never
//! reach it. Set at start and whenever the brand changes.
#![cfg_attr(windows, allow(unsafe_code))]

/// Keeps the icon handle alive while the window uses it.
#[derive(Debug, Default)]
pub(crate) struct TaskbarIcon {
    #[cfg(windows)]
    handle: Option<isize>,
}

impl TaskbarIcon {
    /// Gives the window the brand's icon, big and small.
    pub(crate) fn apply(&mut self, frame: &eframe::Frame) {
        if let Err(err) = self.set(frame) {
            tracing::warn!(%err, "could not set the taskbar icon");
        }
    }

    #[cfg(windows)]
    fn set(&mut self, frame: &eframe::Frame) -> Result<(), String> {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            CreateIcon, DestroyIcon, ICON_BIG, ICON_SMALL, SendMessageW, WM_SETICON,
        };

        const SIZE: u32 = 64;
        let handle = frame.window_handle().map_err(|e| e.to_string())?;
        let RawWindowHandle::Win32(window) = handle.as_raw() else {
            return Err("not a Win32 window".into());
        };
        let hwnd = window.hwnd.get() as windows_sys::Win32::Foundation::HWND;
        // Windows wants BGRA rows; alpha comes from the 32-bit colour bits,
        // so the AND mask is all zeros.
        let mut bgra = crate::icon::rgba(SIZE);
        for px in bgra.as_chunks_mut::<4>().0 {
            px.swap(0, 2);
        }
        let mask = vec![0u8; (SIZE * SIZE / 8) as usize];
        // SAFETY: both buffers have the sizes CreateIcon reads for a
        // SIZE × SIZE icon at 32 bits per pixel.
        let icon = unsafe {
            CreateIcon(
                std::ptr::null_mut(),
                SIZE as i32,
                SIZE as i32,
                1,
                32,
                mask.as_ptr(),
                bgra.as_ptr(),
            )
        };
        if icon.is_null() {
            return Err(std::io::Error::last_os_error().to_string());
        }
        // SAFETY: `hwnd` is this process's live top-level window, from winit,
        // and `icon` a valid icon handle made above.
        unsafe {
            SendMessageW(hwnd, WM_SETICON, ICON_BIG as usize, icon as isize);
            SendMessageW(hwnd, WM_SETICON, ICON_SMALL as usize, icon as isize);
        }
        if let Some(old) = self.handle.replace(icon as isize) {
            // SAFETY: the previous icon made here, no longer used by the window.
            unsafe { DestroyIcon(old as windows_sys::Win32::UI::WindowsAndMessaging::HICON) };
        }
        Ok(())
    }

    #[cfg(not(windows))]
    fn set(&mut self, _frame: &eframe::Frame) -> Result<(), String> {
        // Elsewhere the window icon eframe sets is the one the dock and the
        // task switcher show.
        Ok(())
    }
}
