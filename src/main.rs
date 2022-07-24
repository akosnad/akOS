use std::{path::Path, process::Command};

fn main() {
    let _uefi_fat_path = Path::new(env!("UEFI_FAT_PATH"));
    let uefi_gpt_path = Path::new(env!("UEFI_GPT_PATH"));

    // TODO: provide this through a crate
    let omvf_path = &Path::new("OVMF-pure-efi.fd");
    run_in_qemu(uefi_gpt_path, omvf_path);
}

fn run_in_qemu(uefi_gpt_path: &Path, omvf_path: &Path) {
    let mut cmd = Command::new("qemu-system-x86_64");
    cmd.arg("-drive");
    cmd.arg(format!("format=raw,file={}", uefi_gpt_path.display()));
    cmd.arg("-bios").arg(omvf_path);

    let status = cmd.status().unwrap().code().or(Some(1)).unwrap();
    std::process::exit(status);
}
