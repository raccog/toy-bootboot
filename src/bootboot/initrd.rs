use alloc::vec::Vec;

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
}
