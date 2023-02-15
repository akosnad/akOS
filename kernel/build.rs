#![feature(custom_inner_attributes)]
#![rustfmt::skip::macros(command)]

use std::env;

#[macro_use]
extern crate command_macros;

fn cargo_version() -> String {
    let mut cmd = command!(cargo --version);
    let out = cmd.output().expect("cannot get cargo version");
    String::from_utf8(out.stdout).expect("failed to parse cargo version")
}

fn rustc_version() -> String {
    let mut cmd = command!(rustc --version);
    let out = cmd.output().expect("cannot get rustc version");
    String::from_utf8(out.stdout).expect("failed to parse rustc version")
}

fn setup_info_vars() {
    println!(
        "cargo:rustc-env=BUILD_TARGET={}",
        env::var("TARGET").expect("cannot get build target")
    );
    println!(
        "cargo:rustc-env=BUILD_DATE={}",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
    );
    println!(
        "cargo:rustc-env=PROFILE={}",
        env::var("PROFILE").expect("cannot get build profile")
    );
    println!("cargo:rustc-env=CARGO_VERSION={}", cargo_version());
    println!("cargo:rustc-env=RUSTC_VERSION={}", rustc_version());
}

fn main() {
    let cwd_path = env::current_dir().expect("cannot get current directory");
    let cwd = cwd_path.display();
    println!("cargo:rerun-if-changed={cwd}/build.rs");
    println!("cargo:rerun-if-changed={cwd}/Cargo.toml");
    println!("cargo:rerun-if-changed={cwd}/Cargo.lock");
    println!("cargo:rerun-if-changed={cwd}/src");
    println!("cargo:rerun-if-changed={cwd}/linker.ld");

    setup_info_vars();
    println!("cargo:rustc-link-arg=-T{cwd}/linker.ld");
}
