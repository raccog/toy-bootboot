use alloc::vec::Vec;

use uefi::{
    proto::media::file::{Directory, File, FileAttribute, FileMode, RegularFile},
    Result as UefiResult,
};

mod ustar;

use crate::{open_file, read_to_vec};
use ustar::read_ustar;

/// Reads initrd file from boot partition.
///
/// The default file path is `BOOTBOOT/INITRD`.
///
/// # Panics
///
/// If the initrd file cannot be found.
pub fn get_initrd(bootdir: &mut Directory) -> Initrd {
    // Initrd file
    const INITRD_ERR: &str = "Initrd not found";
    let mut initrd_file = get_initrd_file(bootdir).expect(INITRD_ERR);

    // Read initrd
    let initrd = read_to_vec(&mut initrd_file).expect(INITRD_ERR);
    let initrd = Initrd::new(initrd);

    // Close initrd file
    initrd_file.close();

    initrd
}

/// BOOTBOOT initrd
#[repr(C)]
#[derive(Clone)]
pub struct Initrd {
    initrd: Vec<u8>,
}

impl Initrd {
    pub fn is_empty(&self) -> bool {
        self.initrd.is_empty()
    }

    pub fn len(&self) -> usize {
        self.initrd.len()
    }

    pub fn new(initrd: Vec<u8>) -> Self {
        Self { initrd }
    }

    pub fn read_file(&self, filename: &str) -> Option<&[u8]> {
        read_ustar(&self.initrd, filename)
    }
}

fn get_initrd_file(bootdir: &mut Directory) -> UefiResult<RegularFile> {
    // Try to open BOOTBOOT/INITRD
    let initrd_file = open_file(bootdir, "INITRD", FileMode::Read, FileAttribute::empty());
    if initrd_file.is_ok() {
        return initrd_file;
    }

    // Try to open BOOTBOOT/X86_64
    open_file(bootdir, "X86_64", FileMode::Read, FileAttribute::empty())
}
