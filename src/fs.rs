use alloc::{boxed::Box, string::String, vec, vec::Vec};
use uefi::{
    prelude::Status,
    proto::media::file::{File, FileInfo, RegularFile},
    Error as UefiError, Result as UefiResult,
};

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
    file.read(&mut buffer[..])
        .map_err(|err| UefiError::new(err.status(), ()))?;

    Ok(buffer)
}

/// Reads an open `file` into a dynamically allocated utf8 `String`
pub fn read_to_string(file: &mut RegularFile) -> UefiResult<String> {
    let buffer = read_to_vec(file)?;
    Ok(String::from_utf8(buffer).map_err(|_| UefiError::new(Status::COMPROMISED_DATA, ()))?)
}
