use log::{debug, error};
use uefi::{
    prelude::*,
    proto::media::file::{Directory, File, FileAttribute, FileMode, RegularFile},
    CString16,
};

/// Read environment from BOOTBOOT/CONFIG in the EFI System Partition.
///
/// # Errors
///
/// An error is returned if the environment is larger than a page of memory (4KiB).
/// This error contains the true size of the config file.
pub fn get_environment(
    image_handle: Handle,
    st: &mut SystemTable<Boot>,
    env: &mut [u8; 4096],
) -> Result<usize, usize> {
    // Boot table
    let bt = st.boot_services();
    // ESP file system
    let fs = unsafe {
        &mut *bt
            .get_image_file_system(image_handle)
            .expect("Failed to get ESP")
            .interface
            .get()
    };
    // Root directory on ESP
    let mut root = fs
        .open_volume()
        .expect("Failed to open root directory on ESP");

    // Read config file
    // TODO: Return error if directory/file cannot be found
    let dirname = CString16::try_from("BOOTBOOT").unwrap();
    let dir = root
        .open(&dirname, FileMode::Read, FileAttribute::DIRECTORY)
        .expect("Could not open BOOTBOOT directory");
    let mut dir = unsafe { Directory::new(dir) };
    let filename = CString16::try_from("CONFIG").unwrap();
    let file = dir
        .open(&filename, FileMode::Read, FileAttribute::empty())
        .expect("Could not open BOOTBOOT/CONFIG file");
    let mut file = unsafe { RegularFile::new(file) };
    // TODO: Return error when file cant be read
    let size = file
        .read(&mut env[..])
        .expect("Could not read BOOTBOOT/CONFIG file");
    file.close();

    // Ensure size of environment is less than a page
    if size > 4096 {
        error!(
            "Size of environment ({} bytes) is larger than a page of memory (4096 bytes)",
            size
        );
        return Err(size);
    }

    // Replace final LF character with null terminator
    env[size - 1] = 0;

    debug!("Successfully read config file of size {}!", size);

    Ok(size)
}
