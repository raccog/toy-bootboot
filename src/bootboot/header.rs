use crate::bootboot::{Framebuffer, Initrd};

/// BOOTBOOT loader type
#[repr(u8)]
pub enum LoaderType {
    BIOS = 0,
    UEFI = 1,
    RPI = 2,
    Coreboot = 3,
}

impl TryFrom<u8> for LoaderType {
    type Error = ();

    fn try_from(loader_type: u8) -> Result<Self, Self::Error> {
        match loader_type {
            0 => Ok(Self::BIOS),
            1 => Ok(Self::UEFI),
            2 => Ok(Self::RPI),
            3 => Ok(Self::Coreboot),
            _ => Err(()),
        }
    }
}

/// BOOTBOOT protocol level
#[repr(u8)]
pub enum BootbootProtocolLevel {
    Static = 1,
    Dynamic = 2,
}

impl TryFrom<u8> for BootbootProtocolLevel {
    type Error = ();

    fn try_from(level: u8) -> Result<Self, Self::Error> {
        match level {
            1 => Ok(Self::Static),
            2 => Ok(Self::Dynamic),
            _ => Err(()),
        }
    }
}

/// BOOTBOOT protocol
#[repr(transparent)]
pub struct BootbootProtocol(u8);

impl BootbootProtocol {
    /// Returns true if this architecture is big endian
    pub fn is_big_endian(&self) -> bool {
        (self.0 & 0x80) == 0x80
    }

    /// Returns the BOOTBOOT protocol level
    pub fn level(&self) -> BootbootProtocolLevel {
        (self.0 & 0b0011).try_into().unwrap()
    }

    /// Returns the BOOTBOOT loader type
    pub fn loader_type(&self) -> LoaderType {
        ((self.0 & 0b0111_1100) >> 2).try_into().unwrap()
    }

    /// Returns a BOOTBOOT protocol
    pub fn new(protocol: u8) -> Self {
        Self(protocol)
    }
}

pub struct BootbootHeader {
    fb: Framebuffer,
    initrd: Initrd,
    protocol: BootbootProtocol,
}

impl BootbootHeader {
    /// Initialize a BOOTBOOT header.
    pub fn new(fb: Framebuffer, initrd: Initrd, protocol: BootbootProtocol) -> Self {
        BootbootHeader {
            fb,
            initrd,
            protocol,
        }
    }

    /// Returns the magic numbers in the BOOTBOOT header.
    ///
    /// Should always be [66, 79, 79, 84], or "BOOT" when read as a string.
    pub fn magic() -> [u8; 4] {
        [66, 79, 79, 84]
    }
}
