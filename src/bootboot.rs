extern crate alloc;
use alloc::{
    string::ToString,
    vec::Vec
};
use core::{
    cmp::Ordering,
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
    mmap: MMapEntry,
}

/// A BOOTBOOT memory map.
pub struct BootbootMMap {
    mmap: Vec<MMapEntry>,
}

impl BootbootMMap {
    /// Converts a UEFI memory map to a BOOTBOOT memory map.
    ///
    /// The memory map entries are also sorted and merged.
    pub fn from_uefi_mmap<'b, MMap>(uefi_mmap: MMap) -> Self
    where
        MMap: ExactSizeIterator<Item = &'b MemoryDescriptor> + Clone,
    {
        // Allocate and convert UEFI memory map
        let mut mmap = Vec::with_capacity(248);
        for desc in uefi_mmap {
            // TODO: Return error if entry fails to be created
            let entry = MMapEntry::new(desc.phys_start, desc.page_count * 4096, MMapEntryType::from_uefi(desc.ty)).unwrap();
            mmap.push(entry);
        }

        // Sort entries
        mmap.sort();

        // Merge entries
        let mut merge_mmap = Vec::with_capacity(mmap.len());
        merge_mmap.push(mmap[0]);
        for entry in mmap[1..].iter() {
            if let Some(merge_entry) = merge_mmap.last().unwrap().merge(entry) {
                *merge_mmap.last_mut().unwrap() = merge_entry;
            } else {
                merge_mmap.push(*entry)
            }
        }
        mmap.clear();
        mmap.extend_from_slice(&merge_mmap);

        Self {
            mmap
        }
    }
}

impl Display for BootbootMMap {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "BootbootMemoryMap (entries: {}):", self.mmap.len())?;
        for entry in self.mmap.iter() {
            write!(
                f,
                "\nAddr: {:08x?} Size: {:08x?} Type: {}",
                entry.ptr,
                entry.size(),
                entry.memory_type().to_string()
            )?;
        }
        Ok(())
    }
}

/// BOOTBOOT memory map entry type.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum MMapEntryType {
    Used = 0,
    Free = 1,
    Acpi = 2,
    Mmio = 3,
    Unknown = 4
}

impl MMapEntryType {
    /// Converts UEFI memory type to BOOTBOOT memory type.
    pub fn from_uefi(ty: MemoryType) -> Self {
       match ty {
            MemoryType::RESERVED
            | MemoryType::RUNTIME_SERVICES_CODE
            | MemoryType::RUNTIME_SERVICES_DATA
            | MemoryType::UNUSABLE
            | MemoryType::PAL_CODE
            | MemoryType::PERSISTENT_MEMORY => Self::Used,
            MemoryType::LOADER_CODE
            | MemoryType::LOADER_DATA
            | MemoryType::BOOT_SERVICES_CODE
            | MemoryType::BOOT_SERVICES_DATA
            | MemoryType::CONVENTIONAL => Self::Free,
            MemoryType::ACPI_RECLAIM | MemoryType::ACPI_NON_VOLATILE => Self::Acpi,
            MemoryType::MMIO | MemoryType::MMIO_PORT_SPACE => Self::Mmio,
            _ => Self::Unknown,
        } 
    }

    /// Creates a memory type from u8 value.
    pub fn new(ty: u8) -> Self {
        match ty {
            0 => Self::Used,
            1 => Self::Free,
            2 => Self::Acpi,
            3 => Self::Mmio,
            _ => Self::Unknown
        }
    }
}

impl<'a> Display for MMapEntryType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}",
               match self {
                   Self::Used => "USED",
                   Self::Free => "FREE",
                   Self::Acpi => "ACPI",
                   Self::Mmio => "Mmio",
                   _ => "UNKNOWN"
               })?;
        Ok(())
    }
}

/// BOOTBOOT memory map entry.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq)]
struct MMapEntry {
    ptr: u64,
    size: u64,
}

impl MMapEntry {
    /// Adds `other`'s size to this entry's size;
    pub fn add_size(&mut self, other: &Self) {
        self.size += other.size() << 4;
    }

    /// Returns true if `other` is the entry directly after this one.
    pub fn is_next(&self, other: &Self) -> bool {
        self.ptr + self.size() == other.ptr && self.memory_type() == other.memory_type()
    }

    /// Returns the type of memory map entry.
    pub fn memory_type(&self) -> MMapEntryType {
        MMapEntryType::new((self.size & 0xf) as u8)
    }

    /// Returns this entry merged with `other` if they are sequential entries.
    ///
    /// Returns `None` if they are `other` is not an entry directly after this one.
    pub fn merge(&self, other: &Self) -> Option<Self> {
        if !self.is_next(other) {
            return None;
        }

        let mut merged = self.clone();
        merged.add_size(other);
        Some(merged)
    }

    /// Create a new BOOTBOOT memory map entry.
    ///
    /// TODO: Return proper error
    pub fn new(ptr: u64, size: u64, ty: MMapEntryType) -> Result<Self, ()> {
        if size > u64::MAX >> 4 {
            return Err(());
        }

        let size = (size << 4) | ty as u64;
        
        Ok(Self {
            ptr,
            size
        })
    }

    /// Returns the size of the entry in bytes.
    pub fn size(&self) -> u64 {
        self.size >> 4
    }
}

impl Ord for MMapEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.ptr.cmp(&other.ptr)
    }
}

impl PartialOrd for MMapEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for MMapEntry {
    fn eq(&self, other: &Self) -> bool {
        self.ptr == other.ptr
    }
}
