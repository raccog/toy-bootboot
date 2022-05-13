use alloc::{string::ToString, vec::Vec};
use core::{
    cmp::Ordering,
    fmt::{self, Display, Formatter},
};
use uefi::table::boot::{MemoryDescriptor, MemoryType};

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
            let entry = MMapEntry::new(
                desc.phys_start,
                desc.page_count * 4096,
                MMapEntryType::from_uefi(desc.ty),
            )
            .unwrap();
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

        Self { mmap }
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
pub enum MMapEntryType {
    Used = 0,
    Free = 1,
    Acpi = 2,
    Mmio = 3,
    Unknown = 4,
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
            _ => Self::Unknown,
        }
    }
}

impl<'a> Display for MMapEntryType {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Used => "USED",
                Self::Free => "FREE",
                Self::Acpi => "ACPI",
                Self::Mmio => "Mmio",
                _ => "UNKNOWN",
            }
        )?;
        Ok(())
    }
}

/// BOOTBOOT memory map entry.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq)]
pub struct MMapEntry {
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

        Ok(Self { ptr, size })
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
