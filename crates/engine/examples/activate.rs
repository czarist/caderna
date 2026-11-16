//! Activates an access code (or licence key) in a data folder, as the app's
//! "Ativar" screen does, and waits for the panel's answer. For setting up
//! test copies of the app (a teacher and a student) without the UI.
//!
//! ```sh
//! cargo run -p kg-engine --example activate -- <code> [data folder]
//! ```
//!
//! Without a folder it uses this build's standard data folder, the one the
//! installed app of the same channel opens.

use std::time::{Duration, Instant};

use kg_engine::{AiConfig, AppPaths, Command, Engine, PanelStatus};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let code = args.next().ok_or("usage: activate <code> [data folder]")?;
    let paths = args.next().map_or_else(AppPaths::standard, AppPaths::new);
    println!(
        "panel: {}, data folder: {}",
        kg_brand::SERVER_URL,
        paths.data_dir.display()
    );

    let mut engine = Engine::new(paths, AiConfig::default())?;
    engine.handle(Command::ActivateLicense(code))?;
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(60)
        && !matches!(engine.panel_status(), PanelStatus::Connected { .. })
    {
        engine.poll();
        std::thread::sleep(Duration::from_millis(100));
    }
    for notice in engine.take_notices() {
        println!("  notice: {notice:?}");
    }
    println!(
        "status: {:?}, role {:?}",
        engine.panel_status(),
        engine.role()
    );
    match engine.panel_status() {
        PanelStatus::Connected { .. } => Ok(()),
        other => Err(format!("not activated: {other:?}").into()),
    }
}
