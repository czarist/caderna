//! Serializes shader compiler invocations without modifying the pinned llama.cpp.
//! Build scripts set KG_REAL_GLSLC and KG_SHADER_LOCK for the child process.
use std::{
    fs::OpenOptions,
    process::{Command, ExitCode},
};

fn run() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let compiler = std::env::var_os("KG_REAL_GLSLC").ok_or("KG_REAL_GLSLC is missing")?;
    let lock_path = std::env::var_os("KG_SHADER_LOCK").ok_or("KG_SHADER_LOCK is missing")?;
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(lock_path)?;
    lock.lock()?;
    let status = Command::new(compiler)
        .args(std::env::args_os().skip(1))
        .status()?;
    Ok(if status.success() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("shader compiler wrapper: {error}");
            ExitCode::FAILURE
        }
    }
}
