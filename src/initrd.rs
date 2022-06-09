use alloc::vec::Vec;
use log::debug;
use uefi::{
    proto::media::file::{Directory, File, FileAttribute, FileMode, RegularFile},
    Result as UefiResult,
};

mod ustar;

use crate::{open_file, read_to_vec};
use ustar::read_ustar;

/// BOOTBOOT initrd.
#[repr(C)]
#[derive(Clone)]
pub struct Initrd {
    initrd_raw: Vec<u8>,
}

impl Initrd {
    /// Reads initrd file from boot partition.
    ///
    /// The following files are read in order until one is a valid file:
    ///
    /// * `BOOTBOOT/INITRD`
    /// * `BOOTBOOT/X86_64`
    ///
    /// # Errors
    ///
    /// Returns an error if initrd file could not be read to memory.
    pub fn from_disk(bootdir: &mut Directory) -> UefiResult<Self> {
        // Initrd file
        let mut initrd_file = get_initrd_file(bootdir)?;

        // Read initrd
        let initrd_raw = read_to_vec(&mut initrd_file)?;
        let initrd = Self { initrd_raw };

        // Close initrd file
        initrd_file.close();

        Ok(initrd)
    }

    /// Tries to read `filename` from initrd using various file system types.
    ///
    /// Returns `None` if `filename` does not exist.
    ///
    /// Currently the only supported file system is ustar.
    pub fn read_file(&self, filename: &str) -> Option<&[u8]> {
        read_ustar(&self.initrd_raw, filename)
    }

    /// Returns the initrd file's size in bytes.
    pub fn size(&self) -> usize {
        self.initrd_raw.len()
    }
}

/// Searches `BOOTBOOT/INITRD` and `BOOTBOOT/X86_64` for initrd file.
///
/// # Errors
///
/// Returns error if intird could not be found in either file.
fn get_initrd_file(bootdir: &mut Directory) -> UefiResult<RegularFile> {
    // Try to open BOOTBOOT/INITRD
    let initrd_file = open_file(bootdir, "INITRD", FileMode::Read, FileAttribute::empty());
    if initrd_file.is_ok() {
        debug!("Found initrd in 'BOOTBOOT/INITRD'");
        return initrd_file;
    }

    // Try to open BOOTBOOT/X86_64
    let initrd_file = open_file(bootdir, "X86_64", FileMode::Read, FileAttribute::empty());
    if initrd_file.is_ok() {
        debug!("Found initrd in 'BOOTBOOT/X86_64'");
    }
    initrd_file
}
