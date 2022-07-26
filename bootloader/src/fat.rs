use anyhow::Context;
use std::{collections::BTreeMap, fs, io, path::Path};
use crate::KERNEL_FILE_NAME;

pub fn create_fat_filesystem(files: BTreeMap<&str, &Path>, out: &Path) -> anyhow::Result<()> {
    const MB: u64 = 1024 * 1024;

    // calculate needed size
    let mut needed_size = 0;
    for path in files.values() {
        let file_size = fs::metadata(path)
            .with_context(|| format!("failed to read metadata of file `{}`", path.display()))?
            .len();
        needed_size += file_size;
    }

    // create new filesystem image file at the given path and set its length
    let fat_file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&out)
        .unwrap();
    let fat_size_padded_and_rounded = ((needed_size + 1024 * 64 - 1) / MB + 1) * MB;
    fat_file.set_len(fat_size_padded_and_rounded).unwrap();

    // choose a file system label
    let mut label = *b"MY_RUST_OS!";
    if let Some(path) = files.get(KERNEL_FILE_NAME) {
        if let Some(name) = path.file_stem() {
            let converted = name.to_string_lossy();
            let name = converted.as_bytes();
            let mut new_label = [0u8; 11];
            let name = &name[..new_label.len()];
            let slice = &mut new_label[..name.len()];
            slice.copy_from_slice(name);
            label = new_label;
        }
    }

    // format the file system and open it
    let format_options = fatfs::FormatVolumeOptions::new().volume_label(label);
    fatfs::format_volume(&fat_file, format_options).context("Failed to format UEFI FAT file")?;
    let filesystem = fatfs::FileSystem::new(&fat_file, fatfs::FsOptions::new())
        .context("Failed to open FAT file system of UEFI FAT file")?;

    // copy files to file system
    let root_dir = filesystem.root_dir();
    for (target_path_raw, file_path) in files {
        let target_path = Path::new(target_path_raw);
        // create parent directories
        let ancestors: Vec<_> = target_path.ancestors().skip(1).collect();
        for ancestor in ancestors.into_iter().rev().skip(1) {
            root_dir
                .create_dir(&ancestor.display().to_string())
                .with_context(|| {
                    format!(
                        "failed to create directory `{}` on FAT filesystem",
                        ancestor.display()
                    )
                })?;
        }

        let mut new_file = root_dir
            .create_file(target_path_raw)
            .with_context(|| format!("failed to create file at `{}`", target_path.display()))?;
        new_file.truncate().unwrap();
        io::copy(
            &mut fs::File::open(file_path)
                .with_context(|| format!("failed to open `{}` for copying", file_path.display()))?,
            &mut new_file,
        )
        .with_context(|| format!("failed to copy `{}` to FAT filesystem", file_path.display()))?;
    }

    Ok(())
}


