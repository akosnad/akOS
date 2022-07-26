use ak_os_bootloader_common::Kernel;
use core::{ptr, slice};
use uefi::{
    prelude::{Boot, Handle, SystemTable},
    proto::{
        device_path::DevicePath,
        loaded_image::LoadedImage,
        media::{
            file::{File, FileAttribute, FileInfo, FileMode},
            fs::SimpleFileSystem,
        },
    },
    table::boot::{
        AllocateType, MemoryType, OpenProtocolAttributes, OpenProtocolParams,
    },
    CStr16,
};


// Taken from rust-osdev/bootloader
// Copyright (c) 2018 Philipp Oppermann
pub fn load_kernel(image: Handle, st: &SystemTable<Boot>) -> Kernel<'static> {
    let slice = load_kernel_file(image, st).expect("couldn't find kernel");
    Kernel::parse(slice)
}

fn load_kernel_file(image: Handle, st: &SystemTable<Boot>) -> Option<&'static mut [u8]> {
    let file_system_raw = {
        let ref this = st.boot_services();
        let loaded_image = this
            .open_protocol::<LoadedImage>(
                OpenProtocolParams {
                    handle: image,
                    agent: image,
                    controller: None,
                },
                OpenProtocolAttributes::Exclusive,
            )
            .expect("Failed to retrieve `LoadedImage` protocol from handle");
        let loaded_image = unsafe { &*loaded_image.interface.get() };

        let device_handle = loaded_image.device();

        let device_path = this
            .open_protocol::<DevicePath>(
                OpenProtocolParams {
                    handle: device_handle,
                    agent: image,
                    controller: None,
                },
                OpenProtocolAttributes::Exclusive,
            )
            .expect("Failed to retrieve `DevicePath` protocol from image's device handle");
        let mut device_path = unsafe { &*device_path.interface.get() };

        let fs_handle = this
            .locate_device_path::<SimpleFileSystem>(&mut device_path)
            .ok()?;

        this.open_protocol::<SimpleFileSystem>(
            OpenProtocolParams {
                handle: fs_handle,
                agent: image,
                controller: None,
            },
            OpenProtocolAttributes::Exclusive,
        )
    }
    .unwrap();
    let file_system = unsafe { &mut *file_system_raw.interface.get() };

    let mut root = file_system.open_volume().unwrap();
    let mut buf = [0; 14 * 2];
    let filename = CStr16::from_str_with_buf("kernel-x86_64", &mut buf).unwrap();
    let kernel_file_handle = root
        .open(filename, FileMode::Read, FileAttribute::empty())
        .expect("Failed to load kernel (expected file named `kernel-x86_64`)");
    let mut kernel_file = match kernel_file_handle.into_type().unwrap() {
        uefi::proto::media::file::FileType::Regular(f) => f,
        uefi::proto::media::file::FileType::Dir(_) => panic!(),
    };

    let mut buf = [0; 500];
    let kernel_info: &mut FileInfo = kernel_file.get_info(&mut buf).unwrap();
    let kernel_size = usize::try_from(kernel_info.file_size()).unwrap();

    let kernel_ptr = st
        .boot_services()
        .allocate_pages(
            AllocateType::AnyPages,
            MemoryType::LOADER_DATA,
            ((kernel_size - 1) / 4096) + 1,
        )
        .unwrap() as *mut u8;
    unsafe { ptr::write_bytes(kernel_ptr, 0, kernel_size) };
    let kernel_slice = unsafe { slice::from_raw_parts_mut(kernel_ptr, kernel_size) };
    kernel_file.read(kernel_slice).unwrap();

    Some(kernel_slice)
}
