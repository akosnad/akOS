use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    export_build_info();
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=Cargo.lock");
    println!("cargo:rerun-if-changed=common");

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    println!("cargo:rustc-env=UEFI_BOOTLOADER_PATH={}", build_uefi_bootloader(&out_dir).display());
}

fn build_uefi_bootloader(out_dir: &Path) -> PathBuf {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let mut cmd = Command::new(cargo);
    cmd.arg("install");
    #[cfg(debug_assertions)]
    { cmd.arg("--debug"); }
    cmd.arg("--path").arg("uefi");
    println!("cargo:rerun-if-changed=uefi");
    cmd.arg("--target-dir").arg("target");
    cmd.arg("--locked");
    cmd.arg("--target").arg("x86_64-unknown-uefi");
    cmd.arg("-Zbuild-std=core,alloc")
        .arg("-Zbuild-std-features=compiler-builtins-mem");
    cmd.arg("--root").arg(out_dir);
    cmd.env_remove("RUSTFLAGS");
    cmd.env_remove("CARGO_ENCODED_RUSTFLAGS");
    let status = cmd
        .status()
        .expect("failed to build uefi bootloader");
    if status.success() {
        let path = out_dir.join("bin").join("ak_os-bootloader-uefi.efi");
        assert!(
            path.exists(),
            "uefi bootloader executable does not exist after building"
        );
        path
    } else {
        panic!("failed to build uefi bootloader");
    }
}

fn export_build_info() {
    let git_out = Command::new("git").args(&["rev-parse", "HEAD"]).output().unwrap();
    let git_hash = String::from_utf8(git_out.stdout).unwrap();
    std::env::set_var("BUILD_GIT_HASH", &git_hash);

    let utc_build_time = build_time::build_time_utc!("%Y-%m-%d %H:%M:%S UTC");
    std::env::set_var("BUILD_TIME", utc_build_time);

    let hostname_out = Command::new("hostname").output().unwrap();
    let _build_host = String::from_utf8(hostname_out.stdout).unwrap();
    let build_host = _build_host.trim();
    std::env::set_var("BUILD_HOST", build_host);
}
