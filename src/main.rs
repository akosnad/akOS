use std::{path::Path, process::Command};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let debug: bool = args.contains(&String::from("-d")) || args.contains(&String::from("--debug"));
    let bios: bool = args.contains(&String::from("-b")) || args.contains(&String::from("--bios"));
    
    if bios {
        let bios_image_path = Path::new(env!("BIOS_GPT_PATH"));
        run_in_qemu(bios_image_path, None, debug);
    } else {
        let uefi_image_path = Path::new(env!("UEFI_GPT_PATH"));
        let omvf_path = &Path::new("OVMF-pure-efi.fd");
        run_in_qemu(uefi_image_path, Some(omvf_path), debug);
    }
}

fn run_in_qemu(uefi_gpt_path: &Path, omvf_path: Option<&Path>, debug: bool) {
    let mut cmd = Command::new("qemu-system-x86_64");
    cmd.arg("-drive");
    cmd.arg(format!("format=raw,file={}", uefi_gpt_path.display()));
    if let Some(omvf_path) = omvf_path {
        cmd.arg("-bios").arg(omvf_path);
    }
    if debug { cmd.arg("-s").arg("-S"); }

    let status = cmd.status().unwrap().code().or(Some(1)).unwrap();
    std::process::exit(status);
}
