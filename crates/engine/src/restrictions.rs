//! What a school blocks for its students in the panel's client area. Without
//! a panel nothing is blocked.

use crate::Command;

/// Something the school can block for its students.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Restriction {
    /// Saving handouts outside the app or opening them in another program.
    ExportHandouts,
    /// Screenshots and screen recordings of the app's window.
    ScreenCapture,
}

impl Restriction {
    /// The panel's id (`App\Enums\StudentRestriction`).
    pub fn id(self) -> &'static str {
        match self {
            Self::ExportHandouts => "export_handouts",
            Self::ScreenCapture => "screen_capture",
        }
    }
}

impl Command {
    /// What a school may block about this command.
    pub fn restriction(&self) -> Option<Restriction> {
        match self {
            Self::ExportReading(_) | Self::ExportSavedReading { .. } => {
                Some(Restriction::ExportHandouts)
            }
            _ => None,
        }
    }
}
