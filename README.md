# akOS
Rust hobby UEFI OS/kernel

(only a kernel at it's current state)

## Project structure
```
akOS
├─── build.rs            (main build script)
├─── src 
|   └─── main.rs         (host runner helper source)
|
├─── kernel
|   ├─── src
|   |   ├─── main.rs     (kernel entrypoint)
|   |   ├─── lib.rs      (kernel library)
|   │   ├─── mem
|   │   ├─── util
|   |   ├─── ...
|   └─── build.rs        (kernel build script)
└─── ...                 (later on: userspace, filesystem, drivers, etc.)
```

## Trying out
Simply:
```
cargo run
```
The project includes a helper application which bootstraps the system and kernel for use with [QEMU](https://www.qemu.org/).
You can also try it out on real hardware with the generated GPT disk image found in `target/build-ak_os-*/akOS.img`.
Please note that MBR and BIOS boot support is experimental, we only fully support UEFI.
