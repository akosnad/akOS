use std::{env, process::Command};

fn cargo_version() -> String {
    let mut cmd = Command::new("cargo");
    cmd.arg("--version");
    let out = cmd.output().unwrap();
    String::from_utf8(out.stdout).unwrap()
}

fn rustc_version() -> String {
    let mut cmd = Command::new("rustc");
    cmd.arg("--version");
    let out = cmd.output().unwrap();
    String::from_utf8(out.stdout).unwrap()
}

fn main() {
    println!(
        "cargo:rustc-env=BUILD_TARGET={}",
        env::var("TARGET").unwrap()
    );
    println!(
        "cargo:rustc-env=BUILD_DATE={}",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
    );
    println!("cargo:rustc-env=PROFILE={}", env::var("PROFILE").unwrap());
    println!("cargo:rustc-env=CARGO_VERSION={}", cargo_version());
    println!("cargo:rustc-env=RUSTC_VERSION={}", rustc_version());
}
