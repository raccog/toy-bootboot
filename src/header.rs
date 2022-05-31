use crate::{Framebuffer, Initrd};

/// BOOTBOOT loader type
#[repr(u8)]
pub enum LoaderType {
    Bios = 0,
    Uefi = 1,
    Rpi = 2,
    Coreboot = 3,
}

impl TryFrom<u8> for LoaderType {
    type Error = ();

    fn try_from(loader_type: u8) -> Result<Self, Self::Error> {
        match loader_type {
            0 => Ok(Self::Bios),
            1 => Ok(Self::Uefi),
            2 => Ok(Self::Rpi),
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
pub struct _BootbootProtocol(u8);

impl _BootbootProtocol {
    /// Returns true if this architecture is big endian
    pub fn _is_big_endian(&self) -> bool {
        (self.0 & 0x80) == 0x80
    }

    /// Returns the BOOTBOOT protocol level
    pub fn _level(&self) -> BootbootProtocolLevel {
        (self.0 & 0b0011).try_into().unwrap()
    }

    /// Returns the BOOTBOOT loader type
    pub fn _loader_type(&self) -> LoaderType {
        ((self.0 & 0b0111_1100) >> 2).try_into().unwrap()
    }

    /// Returns a BOOTBOOT protocol
    pub fn _new(protocol: u8) -> Self {
        Self(protocol)
    }
}

pub struct _BootbootHeader {
    fb: Framebuffer,
    initrd: Initrd,
    protocol: _BootbootProtocol,
}

impl _BootbootHeader {
    /// Initialize a BOOTBOOT header.
    pub fn _new(fb: Framebuffer, initrd: Initrd, protocol: _BootbootProtocol) -> Self {
        _BootbootHeader {
            fb,
            initrd,
            protocol,
        }
    }

    /// Returns the magic numbers in the BOOTBOOT header.
    ///
    /// Should always be [66, 79, 79, 84], or "BOOT" when read as a string.
    pub fn _magic() -> [u8; 4] {
        [66, 79, 79, 84]
    }
}
