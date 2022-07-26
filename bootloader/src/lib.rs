use anyhow::Context;
use std::{collections::BTreeMap, path::Path};

mod fat;
mod gpt;

const KERNEL_FILE_NAME: &str = "kernel-x86_64";

pub fn create_boot_partition(kernel: &Path, out: &Path) -> anyhow::Result<()> {
    let bootloader_path = Path::new(env!("UEFI_BOOTLOADER_PATH"));

    let mut files = BTreeMap::new();
    files.insert("efi/boot/bootx64.efi", bootloader_path);
    files.insert(KERNEL_FILE_NAME, kernel);

    fat::create_fat_filesystem(files, &out).context("failed to create UEFI FAT filesystem")?;

    Ok(())
}

pub fn create_uefi_disk_image(boot_partition: &Path, out: &Path) -> anyhow::Result<()> {
    gpt::create_gpt_disk(boot_partition, out).context("failed to create UEFI disk image")?;

    Ok(())
}
