//! Signs in to the brand's panel with an email and password, as the app's
//! sign-in screen does, then checks that wrong passwords make the panel ask
//! for its captcha. Saves the captcha picture next to the data folder.
//!
//! ```sh
//! KG_BRAND_DIR=branding/caderna KG_CHANNEL=dev \
//!   cargo run -p kg-engine --example sign_in_check -- <email> <password> <data folder>
//! ```
//!
//! Run the wrong-password part last: after two failures the panel asks this
//! address for a captcha for 15 minutes.

use std::time::{Duration, Instant};

use kg_engine::{AiConfig, AppPaths, Command, Engine, PanelStatus};

fn wait(engine: &mut Engine, done: impl Fn(&Engine) -> bool) {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(60) && !done(engine) {
        engine.poll();
        std::thread::sleep(Duration::from_millis(100));
    }
    for notice in engine.take_notices() {
        println!("  notice: {notice:?}");
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: sign_in_check <email> <password> <data folder>";
    let email = args.next().ok_or(usage)?;
    let password = args.next().ok_or(usage)?;
    let folder = std::path::PathBuf::from(args.next().ok_or(usage)?);
    println!("panel: {}", kg_brand::SERVER_URL);

    // 1. The right password: the computer is activated.
    let mut engine = Engine::new(AppPaths::new(folder.join("ok")), AiConfig::default())?;
    engine.handle(Command::SignIn {
        email: email.clone(),
        password: password.clone(),
        captcha: String::new(),
    })?;
    wait(&mut engine, |e| {
        matches!(e.panel_status(), PanelStatus::Connected { .. })
    });
    println!(
        "right password -> status {:?}, role {:?}, licence {}",
        engine.panel_status(),
        engine.role(),
        engine.license().is_some()
    );
    if !matches!(engine.panel_status(), PanelStatus::Connected { .. }) {
        return Err("the right password did not activate".into());
    }

    // 2. Wrong passwords on another folder: the captcha appears.
    let mut engine = Engine::new(AppPaths::new(folder.join("wrong")), AiConfig::default())?;
    for attempt in 1..=2 {
        engine.handle(Command::SignIn {
            email: email.clone(),
            password: "not-the-password".into(),
            captcha: String::new(),
        })?;
        wait(&mut engine, |e| e.panel_status() != PanelStatus::Busy);
        let view = engine.sign_in_view();
        println!(
            "wrong password {attempt} -> needs captcha {}, licence {}",
            view.needs_captcha,
            engine.license().is_some()
        );
    }
    wait(&mut engine, |e| e.sign_in_view().captcha.is_some());
    let view = engine.sign_in_view();
    let Some(captcha) = view.captcha else {
        return Err("no captcha after two wrong passwords".into());
    };
    let picture = folder.join("captcha.png");
    std::fs::write(&picture, &*captcha.image_png)?;
    println!("captcha saved to {}", picture.display());
    if engine.license().is_some() {
        return Err("a wrong password activated the computer".into());
    }
    Ok(())
}
