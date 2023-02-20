//! This is a helper program to run akOS in QEMU
//!
//! It builds the [akOS kernel](../ak_os_kernel) and packages it into a UEFI GPT image.
//! QEMU is then started with a UEFI BIOS and the built the image.
//!
//! # Arguments
//!
//! - `-d` or `--debug`: Run QEMU in debug mode. This will pause the execution of the VM and wait for a GDB connection on port 1234.
//!
//! - `-b` or `--bios`: Run QEMU with a BIOS instead of UEFI. This is useful for debugging BIOS boot issues.
//!
//! - `--`: Any arguments after this will be passed to QEMU. This is useful for passing arguments to QEMU, such as `-smp 4` to run with 4 CPUs.
//!
//! When running with `cargo run`, you can pass arguments to this program by separating them with `--`.
//! For example, `cargo run -- -d -- -smp 4` will run QEMU in debug mode with 4 CPUs.

#![feature(custom_inner_attributes)]
#![rustfmt::skip::macros(command)]
#![feature(custom_test_frameworks)]
#![test_runner(ak_os_tests_runner::test_runner)]
#![reexport_test_harness_main = "test_main"]

use std::path::Path;

#[macro_use]
extern crate command_macros;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let debug: bool = args.contains(&String::from("-d")) || args.contains(&String::from("--debug"));
    let bios: bool = args.contains(&String::from("-b")) || args.contains(&String::from("--bios"));
    let mut extra_args = Vec::new();

    // find extra arguments to pass to QEMU
    if let Some((idx, _)) = args.iter().enumerate().find(|(_, e)| **e == "--") {
        extra_args = args.iter().skip(idx + 1).collect();
    }

    if bios {
        let bios_image_path = Path::new(env!("BIOS_PATH"));
        run_in_qemu(bios_image_path, None, debug, extra_args);
    } else {
        let uefi_image_path = Path::new(env!("UEFI_PATH"));
        let omvf_path = &Path::new("OVMF-pure-efi.fd");
        run_in_qemu(uefi_image_path, Some(omvf_path), debug, extra_args);
    }
}

fn run_in_qemu(
    uefi_gpt_path: &Path,
    omvf_path: Option<&Path>,
    debug: bool,
    extra_args: Vec<&String>,
) {
    let mut cmd =
        command!(qemu-system-x86_64 -serial stdio -device isa-debug-exit,iobase=0xf4,iosize=0x04);
    cmd.arg("-drive")
        .arg(format!("format=raw,file={}", uefi_gpt_path.display()));
    if let Some(omvf_path) = omvf_path {
        cmd.arg("-bios").arg(omvf_path);
    }
    if debug {
        println!(
            "Running in debug mode, booted image is at {}",
            uefi_gpt_path.display()
        );
        cmd.arg("-s").arg("-S");
    }
    for arg in extra_args {
        cmd.arg(arg);
    }

    let status = cmd
        .status()
        .expect("failed to run qemu")
        .code()
        .unwrap_or(1);
    std::process::exit(status);
}
