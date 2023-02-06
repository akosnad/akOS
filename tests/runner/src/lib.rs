#![feature(pattern)]
#![feature(custom_test_frameworks)]
#![test_runner(crate::test_runner)]
#![reexport_test_harness_main = "test_main"]

#[macro_use]
extern crate command_macros;

use bootloader::UefiBoot;
use regex::Regex;
use std::{
    env::var,
    ffi::OsString,
    fs, io,
    path::{Path, PathBuf},
    process::Stdio,
};

pub trait Testable {
    fn run(&self);
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        println!("{}...", core::any::type_name::<T>());
        self();
        println!("[ok]");
    }
}

const OVMF_PATH: &str = "OVMF-pure-efi.fd";

pub fn test_runner(tests: &[&dyn Testable]) {
    let target_dir: String = env!("OUT_DIR").to_string();
    let tests_dir: String = format!("{target_dir}/x86_64-unknown-none/debug/deps");

    println!("Compiling kernel and integration tests...");
    compile_kernel_and_tests(&target_dir);
    let (kernel_integration_tests, kernel) = find_kernel_tests(&tests_dir).unwrap();

    let total_kernel_tests = kernel_integration_tests.len();
    let mut succeeded_kernel_tests = 0;
    let mut failed_kernel_tests = 0;

    println!("Running {total_kernel_tests} kernel integration tests...");

    for test in kernel_integration_tests {
        let disk = build_test_disk(
            &target_dir,
            Path::new(&tests_dir).join(test.clone()).as_path(),
        );
        match run_in_qemu(disk.as_path(), Path::new(OVMF_PATH)) {
            Ok(_) => succeeded_kernel_tests += 1,
            Err(_) => failed_kernel_tests += 1,
        }
    }

    println!("Kernel integration test results: {total_kernel_tests} total, {succeeded_kernel_tests} succeeded, {failed_kernel_tests} failed");

    println!("Runnig kernel unit tests...");
    let disk = build_test_disk(&target_dir, Path::new(&tests_dir).join(&kernel).as_path());
    run_in_qemu(&disk, Path::new(OVMF_PATH)).ok();

    println!("Running other {} tests", tests.len());
    for test in tests {
        test.run();
    }
}

fn compile_kernel_and_tests(target_dir: &str) {
    const CARGO: &str = "cargo";

    #[allow(clippy::needless_borrow)]
    let mut cmd = command!((var("CARGO").unwrap_or(CARGO.into())) build --package ak_os-kernel --target x86_64-unknown-none --test=* --target-dir=(target_dir) --no-default-features --features=test);
    //cmd.env("RUSTFLAGS", "--test");

    let status = cmd.status().unwrap();
    assert!(status.success());
}

fn get_files(dir: &Path) -> Result<Vec<OsString>, io::Error> {
    let v: Vec<OsString> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter_map(|e| e.path().file_name().map(|s| s.to_os_string()))
        .collect();
    Ok(v)
}

fn find_kernel_tests(tests_dir: &str) -> Result<(Vec<OsString>, OsString), io::Error> {
    let files = get_files(Path::new(tests_dir))?;
    Ok((
        files
            .iter()
            .cloned()
            .filter(|s| {
                let re = Regex::new(r"\-[0-9a-z]{16}$").unwrap();
                s.to_str()
                    .filter(|s| !s.starts_with("ak_os_kernel"))
                    .map(|s| re.is_match(s))
                    .unwrap_or(false)
            })
            .collect(),
        files
            .iter()
            .find(|s| {
                let re = Regex::new(r"^ak_os_kernel\-[0-9a-z]{16}$").unwrap();
                s.to_str().map(|s| re.is_match(s)).unwrap_or(false)
            })
            .unwrap()
            .clone(),
    ))
}

fn build_test_disk(disks_path: &str, test_path: &Path) -> PathBuf {
    let out = Path::new(disks_path).join(test_path).with_extension("img");
    UefiBoot::new(test_path)
        .create_disk_image(out.as_path())
        .expect("failed to create disk image");
    out
}

fn run_in_qemu(uefi_gpt_path: &Path, omvf_path: &Path) -> Result<(), ()> {
    let mut cmd = command!(qemu-system-x86_64 -device isa-debug-exit,iobase=0xf4,iosize=0x04 -display none -serial stdio --no-reboot);

    cmd.arg("-drive")
        .arg(format!("format=raw,file={}", uefi_gpt_path.display()))
        .arg("-bios")
        .arg(omvf_path);

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd.stdin(Stdio::piped());

    let mut child = cmd.spawn().unwrap();

    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();

    let stdout_pipe = std::thread::spawn(move || {
        let reader = std::io::BufReader::new(stdout);
        for line in std::io::BufRead::lines(reader) {
            println!("{}", line.unwrap());
        }
    });

    let stderr_pipe = std::thread::spawn(move || {
        let reader = std::io::BufReader::new(stderr);
        for line in std::io::BufRead::lines(reader) {
            println!("{}", line.unwrap());
        }
    });

    stdout_pipe.join().unwrap();
    stderr_pipe.join().unwrap();

    let status = child.wait().unwrap();
    match status.code() {
        Some(33) => Ok(()),
        _ => Err(()),
    }
}
