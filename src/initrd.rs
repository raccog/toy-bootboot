use alloc::vec::Vec;

mod ustar;

use ustar::read_ustar;

/// BOOTBOOT initrd
#[repr(C)]
#[derive(Clone)]
pub struct Initrd {
    initrd: Vec<u8>,
}

impl Initrd {
    pub fn new(initrd: Vec<u8>) -> Self {
        Self { initrd }
    }

    pub fn read_file(&self, filename: &str) -> Option<&[u8]> {
        read_ustar(&self.initrd, filename)
    }
}
