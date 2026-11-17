//! Live check of developer mode (Phase W9) against the brand's panel:
//! exchanges a code made by an admin, checks the token offline and with the
//! panel, and shows where each environment keeps its data. Nothing is saved.
//!
//! `cargo run -p kg-engine --example developer_live -- DEV-XXXX-XXXX-XXXX`

use kg_engine::developer::{self, Environment};

fn main() {
    let Some(code) = std::env::args().nth(1) else {
        eprintln!("usage: developer_live <DEV-XXXX-XXXX-XXXX>");
        std::process::exit(2);
    };
    println!("panel: {}", kg_brand::SERVER_URL);
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .into();
    let device = developer::this_device();
    let post = |path: &str, body: serde_json::Value| -> (u16, serde_json::Value) {
        match agent
            .post(&format!("{}/api/v1/developer/{path}", kg_brand::SERVER_URL))
            .header("Accept", "application/json")
            .send_json(&body)
        {
            Ok(mut response) => {
                let status = response.status().as_u16();
                let value = response
                    .body_mut()
                    .read_json::<serde_json::Value>()
                    .unwrap_or_default();
                (status, value)
            }
            Err(err) => {
                eprintln!("panel not reached: {err}");
                std::process::exit(1);
            }
        }
    };
    let (status, answer) = post(
        "exchange",
        serde_json::json!({ "code": code, "device": device, "app_version": env!("CARGO_PKG_VERSION") }),
    );
    println!(
        "exchange: {status} {}",
        answer.pointer("/error").unwrap_or(&serde_json::Value::Null)
    );
    let Some(token) = answer.pointer("/data/token").and_then(|t| t.as_str()) else {
        std::process::exit(1);
    };
    match developer::verify(
        token,
        kg_brand::DEVELOPER_PUBLIC_KEY,
        kg_brand::SLUG,
        &device,
        chrono::Utc::now(),
    ) {
        Ok(claims) => println!(
            "offline: ok — {} until {}, environments {:?}",
            claims.person, claims.expires_at, claims.environments
        ),
        Err(err) => println!("offline: refused — {err}"),
    }
    for environment in [Environment::Homologation, Environment::Development] {
        let (status, answer) = post(
            "check",
            serde_json::json!({ "token": token, "environment": environment.id(), "device": device }),
        );
        println!(
            "check {}: {status} active={} reason={}",
            environment.id(),
            answer
                .pointer("/data/active")
                .unwrap_or(&serde_json::Value::Null),
            answer
                .pointer("/data/reason")
                .unwrap_or(&serde_json::Value::Null)
        );
    }
    let standard = kg_engine::AppPaths::standard().data_dir;
    for environment in Environment::ALL {
        println!(
            "{:>4}: {} · {}",
            environment.id(),
            environment.compiled_url(),
            environment.data_dir(&standard).display()
        );
    }
}
