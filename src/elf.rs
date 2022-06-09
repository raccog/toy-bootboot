use core::mem;

/// An error resulting from parsing an ELF file.
#[derive(Copy, Clone, Debug)]
pub enum ElfParseError {
    InvalidAbi,
    InvalidFileType,
    InvalidIsa,
    InvalidMagic,
    InvalidSize,
    InvalidVersion,
    Not64Bit,
    NotLittleEndian,
}

const SIZE_64_BITS: u8 = 2;

const LITTLE_ENDIAN: u8 = 1;

const EXEC_FILE_TYPE: u16 = 2;

const ELF_IDENT_VERSION: u8 = 1;
const ELF_OLD_VERSION: u32 = 1;
const SYSTEMV_ABI: u8 = 0;
const X86_64_ISA: u16 = 0x3e;

/// The header for an ELF64 file.
#[repr(packed)]
#[derive(Copy, Clone, Debug)]
pub struct Elf64Header {
    magic: [u8; 4],
    class: u8,
    data: u8,
    ident_version: u8,
    os_abi: u8,
    _abi_version: u8,
    _padding: [u8; 7],
    file_type: u16,
    isa: u16,
    version: u32,
    entry: u64,
    ph_offset: u64,
    sh_offset: u64,
    _flags: u32,
    header_size: u16,
    _ph_entry_size: u16,
    _ph_num: u16,
    _sh_entry_size: u16,
    _sh_num: u16,
    _sh_string_index: u16,
}

impl Elf64Header {
    /// Returns the entry point to this exectuable.
    pub fn entry(&self) -> u64 {
        self.entry
    }

    /// Returns the valid magic numbers for an ELF header.
    pub fn magic() -> [u8; 4] {
        [0x7f, 0x45, 0x4c, 0x46]
    }

    /// Parses `data` into an ELF64 header and ensures that it is valid and able to be loaded by
    /// this bootloader.
    ///
    /// # Errors
    ///
    ///
    /// * `ElfParseError::InvalidAbi`: ABI is not SystemV
    /// * `ElfParseError::InvalidFileType`: ELF is not executable
    /// * `ElfParseError::InvalidIsa`: ISA is not X86_64
    /// * `ElfParseError::InvalidMagic`: Magic values are invalid
    /// * `ElfParseError::InvalidSize`: ELF header size value does not match real header size
    /// * `ElfParseError::InvalidVersion`: ELF version is not current
    /// * `ElfParseError::Not64Bit`: ELF file is 32bits
    /// * `ElfParseError::NotLittleEndian`: ELF file is big endian
    pub fn new(data: [u8; mem::size_of::<Self>()]) -> Result<Self, ElfParseError> {
        let header = unsafe { *(&data[0] as *const u8 as *const Self) };
        // Ensure magic is valid
        if header.magic != Self::magic() {
            return Err(ElfParseError::InvalidMagic);
        }
        // Ensure file is 64bit
        if header.class != SIZE_64_BITS {
            return Err(ElfParseError::Not64Bit);
        }
        // Ensure little endian, as this only runs on x86_64
        if header.data != LITTLE_ENDIAN {
            return Err(ElfParseError::NotLittleEndian);
        }
        // Ensure ELF is at current version
        if header.ident_version != ELF_IDENT_VERSION && header.version == ELF_OLD_VERSION {
            return Err(ElfParseError::InvalidVersion);
        }
        // Ensure ABI is SystemV
        if header.os_abi != SYSTEMV_ABI {
            return Err(ElfParseError::InvalidAbi);
        }
        // Ensure file type is executable
        if header.file_type != EXEC_FILE_TYPE {
            return Err(ElfParseError::InvalidFileType);
        }
        // Ensure ISA is x86_64
        if header.isa != X86_64_ISA {
            return Err(ElfParseError::InvalidIsa);
        }
        // Ensure header size is valid
        if header.header_size as usize != mem::size_of::<Self>() {
            return Err(ElfParseError::InvalidSize);
        }
        Ok(header)
    }

    /// Returns the offset (from the start of the header) to the first program header.
    pub fn ph_offset(&self) -> u64 {
        self.ph_offset
    }

    /// Returns the offset (from the start of the header) to the first section header.
    pub fn sh_offset(&self) -> u64 {
        self.sh_offset
    }
}
