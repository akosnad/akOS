use std::path::PathBuf;

fn main() {
    let kernel_path = {
        let path = std::env::var("CARGO_BIN_FILE_AK_OS_KERNEL").expect("kernel binary not found");
        PathBuf::from(path)
    };

    let fat_path = kernel_path.with_extension("fat");
    let gpt_path = kernel_path.with_extension("gpt");


    ak_os_bootloader::create_boot_partition(&kernel_path, &fat_path).expect("failed to create boot partition");
    ak_os_bootloader::create_uefi_disk_image(&fat_path, &gpt_path).unwrap();

    println!("cargo:rustc-env=UEFI_FAT_PATH={}", fat_path.display());
    println!("cargo:rustc-env=UEFI_GPT_PATH={}", gpt_path.display());
}
