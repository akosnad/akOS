use std::path::PathBuf;
use std::env::var;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=Cargo.lock");
    println!("cargo:rerun-if-changed=kernel");

    let kernel_path = {
        let path = var("CARGO_BIN_FILE_AK_OS_KERNEL").expect("kernel binary not found");
        PathBuf::from(path)
    };

    let fat_path = {
        let path = var("OUT_DIR").expect("no target dir");
        PathBuf::from(path).with_file_name("akOS.fat")
    };
    let gpt_path = fat_path.with_file_name("akOS.gpt");
    let gpt_path_bios = fat_path.with_file_name("akOS_bios.gpt");


    bootloader::create_boot_partition(&kernel_path, &fat_path).expect("failed to create boot partition");
    bootloader::create_uefi_disk_image(&fat_path, &gpt_path).unwrap();
    bootloader::create_bios_disk_image(&fat_path, &gpt_path_bios).unwrap();

    println!("cargo:rustc-env=UEFI_FAT_PATH={}", fat_path.display());
    println!("cargo:rustc-env=UEFI_GPT_PATH={}", gpt_path.display());
    println!("cargo:rustc-env=BIOS_GPT_PATH={}", gpt_path_bios.display());
}
