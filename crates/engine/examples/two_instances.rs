//! Live check of two data folders on one computer against the brand's panel,
//! as a teacher and a student each running their own copy of the app
//! (`KG_DATA_DIR`): activation by short access code, the role and modules of
//! each, a reopen that needs no code (each folder keeps its own device token
//! in the credential store), then signing both computers out.
//!
//! ```sh
//! cargo run -p kg-engine --example two_instances -- <teacher code> <student code>
//! ```
//!
//! Uses temporary data folders; the sign-outs free the seats and remove the
//! credential-store entries.

use std::time::{Duration, Instant};

use kg_engine::{AiConfig, AppPaths, Command, Engine, Module, PanelStatus};

fn wait(engine: &mut Engine, done: impl Fn(&Engine) -> bool) {
    let start = Instant::now();
    while !done(engine) && start.elapsed() < Duration::from_secs(60) {
        engine.poll();
        std::thread::sleep(Duration::from_millis(100));
    }
    for notice in engine.take_notices() {
        println!("    notice: {notice:?}");
    }
}

fn describe(who: &str, engine: &Engine) {
    println!(
        "  {who}: {:?}, role {:?}, lessons {}, content {}",
        engine.panel_status(),
        engine.role(),
        engine.module_enabled(Module::Lessons),
        engine.module_enabled(Module::Content),
    );
}

fn connected(engine: &Engine) -> bool {
    matches!(engine.panel_status(), PanelStatus::Connected { .. })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if !kg_brand::has_server() {
        return Err("this brand has no [server] url".into());
    }
    let mut args = std::env::args().skip(1);
    let usage = "usage: two_instances <teacher code> <student code>";
    let codes = [args.next().ok_or(usage)?, args.next().ok_or(usage)?];
    let names = ["teacher", "student"];
    let dirs = [tempfile::tempdir()?, tempfile::tempdir()?];
    println!("panel: {}", kg_brand::SERVER_URL);

    let mut engines = Vec::new();
    for ((name, code), dir) in names.iter().zip(&codes).zip(&dirs) {
        let mut engine = Engine::new(AppPaths::new(dir.path()), AiConfig::default())?;
        engine.handle(Command::ActivateLicense(code.clone()))?;
        wait(&mut engine, |e| e.panel_status() != PanelStatus::Busy);
        wait(&mut engine, connected);
        describe(name, &engine);
        if !connected(&engine) {
            return Err(format!("{name} did not connect").into());
        }
        engines.push(engine);
    }

    // Two entries, one per data folder (Windows Credential Manager).
    print_credentials();

    // Close both and reopen: each folder finds its own token, no code asked.
    let before: Vec<_> = engines.iter().map(Engine::panel_status).collect();
    drop(engines);
    let mut engines = Vec::new();
    for (name, dir) in names.iter().zip(&dirs) {
        let mut engine = Engine::new(AppPaths::new(dir.path()), AiConfig::default())?;
        // The start-up check-in with the kept token moves `last_ok` on.
        let earlier = before[engines.len()];
        wait(&mut engine, |e| connected(e) && e.panel_status() != earlier);
        describe(&format!("{name} reopened"), &engine);
        if !connected(&engine) {
            return Err(format!("{name} lost its device token on reopen").into());
        }
        engines.push(engine);
    }

    for (name, engine) in names.iter().zip(engines.iter_mut()) {
        engine.handle(Command::SignOutDevice)?;
        wait(engine, |e| e.panel_status() == PanelStatus::NotActivated);
        describe(&format!("{name} signed out"), engine);
    }
    print_credentials();
    Ok(())
}

fn print_credentials() {
    #[cfg(windows)]
    if let Ok(out) = std::process::Command::new("cmdkey").arg("/list").output() {
        let text = String::from_utf8_lossy(&out.stdout);
        let entries: Vec<&str> = text
            .lines()
            .filter(|line| line.contains("-panel"))
            .map(str::trim)
            .collect();
        println!("  credential entries: {entries:?}");
    }
}
