use std::env::var;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=Cargo.lock");
    println!("cargo:rerun-if-changed=kernel");

    let kernel_path = {
        let path = var("CARGO_BIN_FILE_AK_OS_KERNEL").expect("kernel binary not found");
        PathBuf::from(path)
    };

    let disk_path = {
        let path = var("OUT_DIR").expect("no target dir");
        PathBuf::from(path).with_file_name("akOS.img")
    };
    let disk_path_bios = disk_path.with_file_name("akOS_bios.img");

    let pxe_path = {
        let path = var("OUT_DIR").expect("no target dir");
        PathBuf::from(path).join("pxe")
    };

    let uefi_boot = bootloader::UefiBoot::new(&kernel_path);
    uefi_boot
        .create_disk_image(&disk_path)
        .expect("failed to create boot partition");
    let bios_boot = bootloader::BiosBoot::new(&kernel_path);
    bios_boot
        .create_disk_image(&disk_path_bios)
        .expect("failed to create boot partition");

    uefi_boot
        .create_pxe_tftp_folder(&pxe_path)
        .expect("failed to create pxe tftp folder");

    println!("cargo:rustc-env=UEFI_PATH={}", disk_path.display());
    println!("cargo:rustc-env=BIOS_PATH={}", disk_path_bios.display());
}
