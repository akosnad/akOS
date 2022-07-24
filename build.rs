use std::path::PathBuf;

fn main() {
    let kernel_path = {
        let path = std::env::var("CARGO_BIN_FILE_AK_OS_KERNEL").expect("kernel bin not found");
        PathBuf::from(path)
    };

    let out_fat_path = kernel_path.with_extension("fat");
    let out_gpt_path = kernel_path.with_extension("gpt");

    bootloader::create_uefi_disk_image(&kernel_path, &out_fat_path, &out_gpt_path)
        .expect("failed to build UEFI disk image");

    println!("cargo:rustc-env=UEFI_FAT_PATH={}", out_fat_path.display());
    println!("cargo:rustc-env=UEFI_GPT_PATH={}", out_gpt_path.display());
}
