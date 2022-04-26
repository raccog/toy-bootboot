use core::{
    convert::TryFrom,
    ffi::c_void,
    fmt::{self, Display, Formatter},
    ptr::NonNull,
};
use uefi::table::boot::{AllocateType, BootServices, MemoryDescriptor, MemoryType};

/// BOOTBOOT linear framebuffer
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Framebuffer {
    pub ptr: u64,
    pub size: u32,
    pub width: u32,
    pub height: u32,
    pub scanline: u32,
}

/// BOOTBOOT initrd
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Initrd {
    pub ptr: u64,
    pub size: u64,
}

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

/// A safe wrapper for creating a BOOTBOOT header to pass to the kernel.
pub struct BootbootHeader {
    header: &'static mut BootbootHeaderImpl,
}

impl BootbootHeader {
    /// Initialize a header instance located at `ptr`
    ///
    /// # Note
    ///
    /// The pointer used to cast this header should be page-aligned and have at least a page allocated.
    unsafe fn from_ptr(ptr: *mut c_void) -> Option<Self> {
        NonNull::new(ptr.cast()).map(|mut ptr| Self {
            header: ptr.as_mut(),
        })
    }

    /// Initialize a BOOTBOOT header.
    ///
    /// # Note
    ///
    /// This function allocates a single page to store the BOOTBOOT header.
    pub fn new(
        bt: &BootServices,
        fb: Framebuffer,
        initrd: Initrd,
        protocol: BootbootProtocol,
    ) -> Self {
        let ptr = bt
            .allocate_pages(AllocateType::AnyPages, MemoryType::LOADER_DATA, 1)
            .expect("Could not allocate page for bootboot struct");
        let mut bootboot = unsafe { Self::from_ptr(ptr as *mut c_void).unwrap() };
        bootboot.header.magic[0] = 66;
        bootboot.header.magic[1] = 79;
        bootboot.header.magic[2] = 79;
        bootboot.header.magic[3] = 84;
        bootboot.header.framebuffer = fb;
        bootboot.header.initrd = initrd;
        bootboot.header.protocol = protocol;
        bootboot
    }

    /// Returns the magic numbers in the BOOTBOOT header.
    ///
    /// Should always be [66, 79, 79, 84], or "BOOT" when read as a string.
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
    mmap: MMapEntImpl,
}

/// A BOOTBOOT memory map.
pub struct BootbootMMap<'a> {
    size: u32,
    mmap: &'a [MMapEntImpl],
}

impl<'a> BootbootMMap<'a> {
    /// Converts a UEFI memory map to a BOOTBOOT memory map.
    ///
    /// # Note
    ///
    /// Allocates a single page to store the BOOTBOOT memory map.
    pub fn from_uefi_mmap<'b, MMap>(bt: &BootServices, uefi_mmap: MMap) -> Self
    where
        MMap: ExactSizeIterator<Item = &'b MemoryDescriptor> + Clone,
    {
        let ptr = bt
            .allocate_pages(AllocateType::AnyPages, MemoryType::LOADER_DATA, 1)
            .expect("Could not allocate page for temporary memory map");
        let entries = uefi_mmap.len();
        let mmap = unsafe { (ptr as *mut [MMapEntImpl; 248]).as_mut().unwrap() };

        for (i, desc) in uefi_mmap.enumerate() {
            mmap[i].ptr = desc.phys_start;
            mmap[i].size = (desc.page_count * 4096) << 4;
            mmap[i].size |= match desc.ty {
                MemoryType::RESERVED
                | MemoryType::RUNTIME_SERVICES_CODE
                | MemoryType::RUNTIME_SERVICES_DATA
                | MemoryType::UNUSABLE
                | MemoryType::PAL_CODE
                | MemoryType::PERSISTENT_MEMORY => 0,
                MemoryType::LOADER_CODE
                | MemoryType::LOADER_DATA
                | MemoryType::BOOT_SERVICES_CODE
                | MemoryType::BOOT_SERVICES_DATA
                | MemoryType::CONVENTIONAL => 1,
                MemoryType::ACPI_RECLAIM | MemoryType::ACPI_NON_VOLATILE => 2,
                MemoryType::MMIO | MemoryType::MMIO_PORT_SPACE => 3,
                _ => 0,
            };
        }

        Self {
            size: (entries * 16) as u32,
            mmap: &mmap[0..entries],
        }
    }
}

impl<'a> Display for BootbootMMap<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "BootbootMemoryMap:")?;
        for entry in self.mmap.iter() {
            write!(
                f,
                "\nAddr: {:08x?} Size: {:08x?} Type: {}",
                entry.ptr,
                entry.size >> 4,
                match entry.size & 0xf {
                    0 => "USED",
                    1 => "FREE",
                    2 => "ACPI",
                    3 => "MMIO",
                    _ => "UNKNOWN",
                }
            )?;
        }
        Ok(())
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct MMapEntImpl {
    ptr: u64,
    size: u64,
}
