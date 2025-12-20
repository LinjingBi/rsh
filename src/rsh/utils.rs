use std::error::Error;
use std::fs;
use std::process::Command;

use super::session::AsyncRuntime;

pub fn run_cargo_rsh() -> Result<std::process::Output, Box<dyn Error>> {
    let output = Command::new("cargo")
        .arg("run")
        .arg("--quiet")
        .arg("--bin")
        .arg("__rsh")
        .output()?;
    Ok(output)
}

pub fn looks_like_async_error(stderr: &str) -> bool {
    let patterns = [
        "E0728",
        "E0752",
        "only allowed inside `async` functions",
        "only allowed inside async functions",
        "cannot be used in a `fn` item that is not `async`",
        "future cannot be sent between threads safely",
        "cannot be sent between threads safely",
        "async fn main",
    ];

    patterns.iter().any(|p| stderr.contains(p))
}

pub fn detect_async_runtime() -> Option<AsyncRuntime> {
    let Ok(toml) = fs::read_to_string("Cargo.toml") else {
        return None;
    };
    let lower = toml.to_lowercase();

    if lower.contains("tokio") {
        return Some(AsyncRuntime::Tokio);
    }
    if lower.contains("async-std") {
        return Some(AsyncRuntime::AsyncStd);
    }
    if lower.contains("smol") {
        return Some(AsyncRuntime::Smol);
    }

    None
}

