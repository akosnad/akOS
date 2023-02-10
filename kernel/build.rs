use std::env;

#[macro_use]
extern crate command_macros;

fn cargo_version() -> String {
    let mut cmd = command!(cargo - -version);
    let out = cmd.output().expect("cannot get cargo version");
    String::from_utf8(out.stdout).expect("failed to parse cargo version")
}

fn rustc_version() -> String {
    let mut cmd = command!(rustc - -version);
    let out = cmd.output().expect("cannot get rustc version");
    String::from_utf8(out.stdout).expect("failed to parse rustc version")
}

fn main() {
    println!(
        "cargo:rustc-env=BUILD_TARGET={}",
        env::var("TARGET").expect("cannot get build target")
    );
    println!(
        "cargo:rustc-env=BUILD_DATE={}",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
    );
    println!("cargo:rustc-env=PROFILE={}", env::var("PROFILE").expect("cannot get build profile"));
    println!("cargo:rustc-env=CARGO_VERSION={}", cargo_version());
    println!("cargo:rustc-env=RUSTC_VERSION={}", rustc_version());
}
