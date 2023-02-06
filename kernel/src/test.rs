use crate::{exit_qemu, halt, print, println, QemuExitCode};

pub trait Testable {
    fn run(&self);
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        print!("{}... ", core::any::type_name::<T>());
        self();
        println!("[ok]");
    }
}

pub fn test_runner(tests: &[&dyn Testable]) {
    println!("Running {} tests...", tests.len());
    for test in tests {
        test.run();
    }
}

pub fn test_main() -> ! {
    println!("Kernel booted...");

    // TODO

    exit_qemu(QemuExitCode::Success);
    halt();
}

#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}
