use core::{
    convert::TryFrom,
    ffi::c_void,
    ptr::NonNull,
};
use uefi::{
    table::boot::{AllocateType,BootServices,MemoryType},
};

#[repr(C)]
#[derive(Clone,Copy)]
pub struct Framebuffer {
    pub ptr: u64,
    pub size: u32,
    pub width: u32,
    pub height: u32,
    pub scanline: u32
}

#[repr(C)]
#[derive(Clone,Copy)]
pub struct Initrd {
    pub ptr: u64,
    pub size: u64
}

#[repr(u8)]
pub enum LoaderType {
    BIOS = 0,
    UEFI = 1,
    RPI = 2,
    Coreboot = 3
}

impl TryFrom<u8> for LoaderType {
    type Error = ();

    fn try_from(loader_type: u8) -> Result<Self, Self::Error> {
        match loader_type {
            0 => Ok(Self::BIOS),
            1 => Ok(Self::UEFI),
            2 => Ok(Self::RPI),
            3 => Ok(Self::Coreboot),
            _ => Err(())
        }
    }
}

#[repr(u8)]
pub enum BootbootProtocolLevel {
    Static = 1,
    Dynamic = 2
}

impl TryFrom<u8> for BootbootProtocolLevel {
    type Error = ();

    fn try_from(level: u8) -> Result<Self, Self::Error> {
        match level {
            1 => Ok(Self::Static),
            2 => Ok(Self::Dynamic),
            _ => Err(())
        }
    }
}

#[repr(transparent)]
pub struct BootbootProtocol(u8);

impl BootbootProtocol {
    pub fn is_big_endian(&self) -> bool {
        (self.0 & 0x80) == 0x80
    }

    pub fn level(&self) -> BootbootProtocolLevel {
        (self.0 & 0b0011).try_into().unwrap()
    }

    pub fn loader_type(&self) -> LoaderType {
        ((self.0 & 0b0111_1100) >> 2).try_into().unwrap()
    }

    pub fn new(protocol: u8) -> Self {
        Self(protocol)
    }
}

pub struct BootbootHeader {
    header: &'static mut BootbootHeaderImpl
}

impl BootbootHeader {
    unsafe fn from_ptr(ptr: *mut c_void) -> Option<Self> {
        NonNull::new(ptr.cast()).map(|mut ptr| Self {
            header: ptr.as_mut()
        })
    }

    pub fn new(bt: &BootServices, fb: Framebuffer, initrd: Initrd, protocol: BootbootProtocol) -> Self {
        let ptr = bt.allocate_pages(AllocateType::AnyPages, MemoryType::LOADER_DATA, 1)
            .expect("Could not allocate page for bootboot struct");
        let mut bootboot = unsafe {
            Self::from_ptr(ptr as *mut c_void)
                .unwrap()
        };
        bootboot.header.magic[0] = 66;
        bootboot.header.magic[1] = 79;
        bootboot.header.magic[2] = 79;
        bootboot.header.magic[3] = 84;
        bootboot.header.framebuffer = fb;
        bootboot.header.initrd = initrd;
        bootboot.header.protocol = protocol;
        bootboot
    }

    pub fn magic<'a>(&'a self) -> &'a [u8; 4] {
        &self.header.magic
    }


}

#[repr(C)]
struct BootbootHeaderImpl {
    magic: [u8; 4],
    size: u32,
    protocol: BootbootProtocol,
    fb_type: u8,
    numcores: u16,
    bspid: u16,
    timezone: u16,
    datetime: [u8; 8],
    initrd: Initrd,
    framebuffer: Framebuffer,
    acpi_ptr: u64,
    smbi_ptr: u64,
    efi_ptr: u64,
    mp_ptr: u64,
    _unused0: u64,
    _unused1: u64,
    _unused2: u64,
    _unused3: u64,
    mmap: MMapEntImpl
}

#[repr(C)]
struct MMapEntImpl {
    ptr: u64,
    size: u64
}
