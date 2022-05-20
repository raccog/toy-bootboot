use alloc::{boxed::Box, string::String, vec, vec::Vec};
use uefi::{
    prelude::{ResultExt, Status},
    proto::media::file::{Directory, File, FileAttribute, FileInfo, FileMode, RegularFile},
    CString16, Error as UefiError, Result as UefiResult,
};

/// Opens a subdirectory in the `root` directory.
pub fn open_dir(root: &mut Directory, dirname: &str) -> UefiResult<Directory> {
    let dirname = CString16::try_from(dirname).unwrap();
    root.open(&dirname, FileMode::Read, FileAttribute::DIRECTORY)
        .map(|dir| unsafe { Directory::new(dir) })
}

/// Opens a file in the `root` directory.
pub fn open_file(
    root: &mut Directory,
    filename: &str,
    mode: FileMode,
    attribute: FileAttribute,
) -> UefiResult<RegularFile> {
    let filename = CString16::try_from(filename).unwrap();
    root.open(&filename, mode, attribute)
        .map(|file| unsafe { RegularFile::new(file) })
}

/// Opens a subdirectory in the `root` directory.
///
/// # Panic
///
/// Panics if this directory cannot be opened.
pub fn open_dir_or_panic(root: &mut Directory, dirname: &str) -> Directory {
    let dirname = CString16::try_from(dirname).unwrap();
    let dir = root
        .open(&dirname, FileMode::Read, FileAttribute::DIRECTORY)
        .unwrap_or_else(|_| panic!("Could not open directory named {}", dirname));

    unsafe { Directory::new(dir) }
}

/// Opens a file in the `root` directory.
///
/// # Panic
///
/// Panics if this file cannot be opened.
pub fn open_file_or_panic(
    root: &mut Directory,
    filename: &str,
    mode: FileMode,
    attribute: FileAttribute,
) -> RegularFile {
    let filename = CString16::try_from(filename).unwrap();
    let file = root
        .open(&filename, mode, attribute)
        .unwrap_or_else(|_| panic!("Could not open file named {}", filename));

    unsafe { RegularFile::new(file) }
}

/// Reads an open `file` into a dynamically allocated `Vec<u8>`.
pub fn read_to_vec(file: &mut RegularFile) -> UefiResult<Vec<u8>> {
    // Get file size
    // Returns error if file info cannot be read
    let file_info: Box<FileInfo> = file.get_boxed_info()?;
    let size = file_info.file_size();
    // Allocate buffer
    let mut buffer = vec![0; size as usize];
    // Read file to buffer
    // Returns error if file cannot be read
    file.read(&mut buffer[..]).discard_errdata()?;

    Ok(buffer)
}

/// Reads an open `file` into a dynamically allocated utf8 `String`
pub fn read_to_string(file: &mut RegularFile) -> UefiResult<String> {
    let buffer = read_to_vec(file)?;
    String::from_utf8(buffer)
        .map_err(|_| UefiError::new(Status::COMPROMISED_DATA, ()))
}
