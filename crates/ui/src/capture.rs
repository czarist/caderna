//! Screenshots and screen recordings while the school blocks them for its
//! students (engine [`kg_engine::Restriction::ScreenCapture`]).
//!
//! - Windows leaves the window out of captures (`SetWindowDisplayAffinity`).
//! - macOS marks the window as not shared (`NSWindow.sharingType`), which
//!   older capture APIs respect and newer ones may ignore.
//! - Neither macOS nor Linux tells an app before a screenshot, so there the
//!   content is also covered while the window is out of focus (screenshot
//!   tools usually take the focus) and while a known screen recorder runs.
//!
//! None of this stops a phone camera; the handouts' watermark with the
//! student's name (engine `reader_mark`) is what tells whose copy leaked.
#![cfg_attr(any(windows, target_os = "macos"), allow(unsafe_code))]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Applies the school's choice to the main window when it changes.
#[derive(Debug, Default)]
pub(crate) struct CaptureGuard {
    applied: Option<bool>,
    recorders: Option<RecorderWatch>,
}

/// Why the content is hidden.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Cover {
    /// The window is out of focus.
    Unfocused,
    /// A screen recorder is running (its name).
    Recording(String),
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

    /// What should hide the content this frame; always `None` on Windows,
    /// where the system keeps the window out of captures.
    pub(crate) fn cover(&mut self, ctx: &egui::Context, block: bool) -> Option<Cover> {
        if !block || !COVERS {
            self.recorders = None;
            return None;
        }
        let watch = self
            .recorders
            .get_or_insert_with(|| RecorderWatch::start(ctx.clone()));
        if let Some(name) = watch.running() {
            return Some(Cover::Recording(name));
        }
        if !ctx.input(|input| input.focused) {
            return Some(Cover::Unfocused);
        }
        None
    }
}

/// Whether this system keeps the window out of captures by itself.
pub(crate) const SUPPORTED: bool = cfg!(windows);

/// Whether the content is covered on focus loss and while recorders run.
const COVERS: bool = cfg!(any(target_os = "macos", target_os = "linux"));

/// Paints the cover over the whole window and swallows clicks under it.
pub(crate) fn show_cover(ctx: &egui::Context, cover: &Cover) {
    let screen = ctx.content_rect();
    let palette = crate::theme::palette(ctx);
    let message = match cover {
        Cover::Unfocused => t!("restrictions.covered_unfocused").into_owned(),
        Cover::Recording(name) if name.is_empty() => {
            t!("restrictions.covered_system_capture").into_owned()
        }
        Cover::Recording(name) => {
            t!("restrictions.covered_recording", program = name.as_str()).into_owned()
        }
    };
    egui::Area::new(egui::Id::new("capture_cover"))
        .order(egui::Order::Foreground)
        .fixed_pos(screen.min)
        .show(ctx, |ui| {
            ui.allocate_rect(screen, egui::Sense::click_and_drag());
        });
    // The top layer, so popups and tooltips are hidden too.
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Debug,
        egui::Id::new("capture_cover_paint"),
    ));
    painter.rect_filled(screen, 0.0, palette.background);
    painter.text(
        screen.center(),
        egui::Align2::CENTER_CENTER,
        message,
        egui::FontId::proportional(16.0),
        palette.muted,
    );
}

/// Looks for screen recorders in the background, about once a second.
#[derive(Debug)]
struct RecorderWatch {
    found: Arc<Mutex<Option<String>>>,
    stop: Arc<AtomicBool>,
}

impl RecorderWatch {
    fn start(ctx: egui::Context) -> Self {
        let found = Arc::new(Mutex::new(None));
        let stop = Arc::new(AtomicBool::new(false));
        let (shared, stopped) = (Arc::clone(&found), Arc::clone(&stop));
        let spawned = std::thread::Builder::new()
            .name("recorder-watch".into())
            .spawn(move || {
                while !stopped.load(Ordering::Relaxed) {
                    let now = recorder_in(&process_names()).map(str::to_owned);
                    if let Ok(mut last) = shared.lock()
                        && *last != now
                    {
                        *last = now;
                        ctx.request_repaint();
                    }
                    std::thread::sleep(Duration::from_millis(800));
                }
            });
        if let Err(err) = spawned {
            tracing::warn!(%err, "could not watch for screen recorders");
        }
        Self { found, stop }
    }

    fn running(&self) -> Option<String> {
        self.found.lock().ok().and_then(|found| found.clone())
    }
}

impl Drop for RecorderWatch {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// Screen recorders and screenshot tools by process name (lowercase), with
/// the name shown to the student (empty: the system's own tool).
const RECORDERS: &[(&str, &str)] = &[
    ("obs", "OBS Studio"),
    ("obs64", "OBS Studio"),
    // macOS's own screenshot tool: named in the student's language.
    ("screencaptureui", ""),
    ("screencapture", ""),
    ("quicktime player", "QuickTime Player"),
    ("loom", "Loom"),
    ("cleanshot x", "CleanShot X"),
    ("snagit", "Snagit"),
    ("snagit 2024", "Snagit"),
    ("screenflow", "ScreenFlow"),
    ("kap", "Kap"),
    ("camtasia", "Camtasia"),
    ("simplescreenrecorder", "SimpleScreenRecorder"),
    ("kazam", "Kazam"),
    ("vokoscreen", "vokoscreen"),
    ("vokoscreenng", "vokoscreenNG"),
    ("peek", "Peek"),
    ("kooha", "Kooha"),
    ("recordmydesktop", "recordMyDesktop"),
    ("gpu-screen-recorder", "GPU Screen Recorder"),
    ("wf-recorder", "wf-recorder"),
    ("flameshot", "Flameshot"),
    ("spectacle", "Spectacle"),
    ("ksnip", "ksnip"),
    ("gnome-screenshot", "GNOME Screenshot"),
    ("shutter", "Shutter"),
];

/// The first known recorder among `names`.
fn recorder_in(names: &[String]) -> Option<&'static str> {
    names.iter().find_map(|name| {
        let name = name.trim().to_lowercase();
        RECORDERS
            .iter()
            .find(|(process, _)| *process == name)
            .map(|(_, shown)| *shown)
    })
}

/// Names of the running programs.
fn process_names() -> Vec<String> {
    if cfg!(target_os = "linux") {
        proc_names(std::path::Path::new("/proc"))
    } else if cfg!(target_os = "macos") {
        ps_names()
    } else {
        Vec::new()
    }
}

/// Linux: `/proc/<pid>/comm` of every process.
fn proc_names(proc: &std::path::Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(proc) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .bytes()
                .all(|b| b.is_ascii_digit())
        })
        .filter_map(|entry| std::fs::read_to_string(entry.path().join("comm")).ok())
        .map(|name| name.trim().to_owned())
        .collect()
}

/// macOS: the program names `ps` lists (the last part of each path).
fn ps_names() -> Vec<String> {
    let Ok(output) = std::process::Command::new("/bin/ps")
        .args(["-Axo", "comm="])
        .output()
    else {
        return Vec::new();
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| line.rsplit('/').next().unwrap_or(line).trim().to_owned())
        .collect()
}

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

#[cfg(target_os = "macos")]
fn set_blocked(frame: &eframe::Frame, block: bool) -> Result<(), String> {
    use objc2_app_kit::{NSView, NSWindowSharingType};
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let handle = frame.window_handle().map_err(|e| e.to_string())?;
    let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
        return Err("not an AppKit window".into());
    };
    // SAFETY: winit gives the live NSView of this process's window, and this
    // runs on the main thread (eframe's update).
    let view: &NSView = unsafe { appkit.ns_view.cast::<NSView>().as_ref() };
    let window = view.window().ok_or("the view has no window")?;
    window.setSharingType(if block {
        NSWindowSharingType::None
    } else {
        NSWindowSharingType::ReadOnly
    });
    Ok(())
}

#[cfg(not(any(windows, target_os = "macos")))]
fn set_blocked(_frame: &eframe::Frame, _block: bool) -> Result<(), String> {
    // X11 and Wayland offer no way to leave a window out; see `cover`.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_known_recorders_by_process_name() {
        let names = |list: &[&str]| list.iter().map(|&n| n.to_owned()).collect::<Vec<_>>();
        assert_eq!(recorder_in(&names(&["bash", "Xorg"])), None);
        assert_eq!(recorder_in(&names(&["bash", "obs\n"])), Some("OBS Studio"));
        assert_eq!(
            recorder_in(&names(&["QuickTime Player"])),
            Some("QuickTime Player")
        );
        // A program that only contains a recorder's name is not one.
        assert_eq!(recorder_in(&names(&["jobs-runner"])), None);
    }

    #[test]
    fn reads_process_names_from_proc() {
        let dir = tempfile::tempdir().unwrap();
        for (pid, comm) in [("12", "bash\n"), ("345", "simplescreenrecorder\n")] {
            std::fs::create_dir(dir.path().join(pid)).unwrap();
            std::fs::write(dir.path().join(pid).join("comm"), comm).unwrap();
        }
        std::fs::create_dir(dir.path().join("self")).unwrap();
        let mut names = proc_names(dir.path());
        names.sort();
        assert_eq!(names, ["bash", "simplescreenrecorder"]);
        assert_eq!(recorder_in(&names), Some("SimpleScreenRecorder"));
    }
}
