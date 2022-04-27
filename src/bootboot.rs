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
    mmap: &'a mut [MMapEntImpl],
    is_sorted: bool
}

impl<'a> BootbootMMap<'a> {
    /// Converts a UEFI memory map to a BOOTBOOT memory map.
    ///
    /// The memory map entries are also sorted and merged using [`mergesort`].
    ///
    /// [`mergesort`]: Self::mergesort
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

        let mut mmap = Self {
            size: (entries * 16) as u32,
            mmap: &mut mmap[0..entries],
            is_sorted: false
        };
        mmap.mergesort();
        mmap
    }

    fn sort(mmap: &mut [MMapEntImpl], scratch: &mut [MMapEntImpl]) {
        // Return if there are 0 or 1 memory map entries
        if mmap.len() <= 1 {
            return;
        }

        // Split entries in two
        let middle = mmap.len() / 2;
        {
            let left = &mut mmap[..middle];
            let scratch = &mut scratch[..middle];
            Self::sort(left, scratch);
        }
        {
            let right = &mut mmap[middle..];
            let scratch = &mut scratch[middle..];
            Self::sort(right, scratch);
        }
        Self::merge(mmap, scratch);
    }

    fn merge(mmap: &mut [MMapEntImpl], scratch: &mut [MMapEntImpl]) {
        let middle = mmap.len() / 2;
        let end = mmap.len();
        let mut left_idx = 0;
        let mut right_idx = middle;
        let mut scratch_idx = 0;

        // Merge entries into scratch buffer
        while left_idx < middle && right_idx < end {
            if mmap[left_idx].ptr <= mmap[right_idx].ptr {
                scratch[scratch_idx] = mmap[left_idx];
                left_idx += 1;
            } else {
                scratch[scratch_idx] = mmap[right_idx];
                right_idx += 1;
            }
            scratch_idx += 1;
        }

        // Merge remaining entries
        if left_idx < middle {
            scratch[scratch_idx..].clone_from_slice(&mmap[left_idx..middle]);
        } else if right_idx < end {
            scratch[scratch_idx..].clone_from_slice(&mmap[right_idx..]);
        }

        // Copy scratch buffer to original buffer
        mmap.clone_from_slice(&scratch);
    }

    /// Sorts memory map entries by physical address and merges sequential entries
    /// of the same memory type.
    ///
    /// Uses mergesort for a guaranteed time complexity of `O(n log n)`
    pub fn mergesort(&mut self) {
        // Return if already sorted
        if self.is_sorted {
            return;
        }

        // Return if length is 0 or 1
        if self.mmap.len() <= 1 {
            return;
        }

        // Stack-allocated scratch buffer
        let mut scratch: [MMapEntImpl; 248] = [ MMapEntImpl { ptr: 0, size: 0 }; 248];

        // Begin recursive mergesort
        Self::sort(self.mmap, &mut scratch[..self.mmap.len()]);

        // Merge sequential entries of the same type
        let mut scratch_idx = 0;
        let mut mmap_idx = 0;
        scratch[0] = self.mmap[0];
        while mmap_idx < self.mmap.len() - 1 {
            mmap_idx += 1;
            if scratch[scratch_idx].memory_type() == self.mmap[mmap_idx].memory_type() {
                scratch[scratch_idx].size += self.mmap[mmap_idx].size();
                continue;
            }
            scratch_idx += 1;
            scratch[scratch_idx] = self.mmap[mmap_idx];
        }

        // Copy merged entries and update size
        let scratch_len = scratch_idx + 1;
        self.mmap[..scratch_len].clone_from_slice(&scratch[..scratch_len]);
        self.mmap = self.mmap.take_mut(..scratch_len).unwrap();
        
        self.is_sorted = true;
    }
}

impl<'a> Display for BootbootMMap<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "BootbootMemoryMap (entries: {}):", self.mmap.len())?;
        for entry in self.mmap.iter() {
            write!(
                f,
                "\nAddr: {:08x?} Size: {:08x?} Type: {}",
                entry.ptr,
                entry.size(),
                match entry.memory_type() {
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

impl MMapEntImpl {
    /// Returns the type of memory map entry.
    pub fn memory_type(&self) -> u8 {
        (self.size & 0xf) as u8
    }

    /// Returns the size of the entry in bytes.
    pub fn size(&self) -> u64 {
        self.size >> 4
    }
}
