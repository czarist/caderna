//! Live check of a brand's panel (`[server] url`): activates a licence key on
//! this computer, waits for the check-in and the brand, tests AI through the
//! panel, then signs the computer out again.
//!
//! ```sh
//! cargo run -p kg-engine --example panel_check -- <licence key file>
//! ```
//!
//! Uses a temporary data folder; the device token goes to the operating
//! system's credential store and is removed by the sign-out.

use std::time::{Duration, Instant};

use kg_engine::{AiConfig, AppPaths, Command, Engine, PanelStatus};

fn wait(engine: &mut Engine, what: &str, done: impl Fn(&Engine) -> bool) {
    let start = Instant::now();
    while !done(engine) && start.elapsed() < Duration::from_secs(60) {
        engine.poll();
        std::thread::sleep(Duration::from_millis(100));
    }
    for notice in engine.take_notices() {
        println!("  notice: {notice:?}");
    }
    println!("{what}: {:?}", engine.panel_status());
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if !kg_brand::has_server() {
        return Err("this brand has no [server] url".into());
    }
    let file = std::env::args()
        .nth(1)
        .ok_or("usage: panel_check <licence key file>")?;
    let key = std::fs::read_to_string(file)?;
    let dir = tempfile::tempdir()?;
    let mut engine = Engine::new(AppPaths::new(dir.path()), AiConfig::default())?;
    println!("panel: {}", kg_brand::SERVER_URL);
    println!(
        "before: {:?}, app {:?}",
        engine.panel_status(),
        engine.demo()
    );

    engine.handle(Command::ActivateLicense(key))?;
    wait(&mut engine, "activated", |e| {
        e.panel_status() != PanelStatus::Busy
    });
    println!("app: {:?}", engine.demo());

    // The brand downloaded by the check-in applies from the next start.
    let reopened = Engine::new(AppPaths::new(dir.path()), AiConfig::default())?;
    match reopened.panel_brand() {
        Some(brand) => {
            let pictures: Vec<String> = brand
                .images
                .iter()
                .map(|(file, bytes)| format!("{file} ({} bytes)", bytes.len()))
                .collect();
            println!(
                "brand at the next start: {:?} by {:?}, accent {:?}, pictures {pictures:?}, texts {:?}",
                brand.app_name, brand.company_name, brand.accent_light, brand.texts
            );
        }
        None => println!("brand at the next start: none"),
    }
    drop(reopened);

    engine.handle(Command::TestConnection)?;
    let ai = engine.ai_config();
    wait(&mut engine, "AI tested", |e| {
        e.connection_result(&ai).is_some()
    });
    println!(
        "AI through the panel works: {:?}",
        engine.connection_result(&ai)
    );

    engine.handle(Command::SignOutDevice)?;
    wait(&mut engine, "signed out", |e| {
        e.panel_status() == PanelStatus::NotActivated
    });
    println!(
        "after: app {:?}, licence kept: {}",
        engine.demo(),
        engine.license().is_some()
    );
    Ok(())
}
