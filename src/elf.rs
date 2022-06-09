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

const ELF_HEADER_NIDENT: usize = 16;

const SIZE_64_BITS: u8 = 2;

const LITTLE_ENDIAN: u8 = 1;

const EXEC_FILE_TYPE: u16 = 2;

const ELF_IDENT_VERSION: u8 = 1;
const ELF_OLD_VERSION: u32 = 1;
const SYSTEMV_ABI: u8 = 0;
const X86_64_ISA: u16 = 0x3e;

/// The header for an ELF64 file.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Elf64Header {
    ident: [u8; ELF_HEADER_NIDENT],
    file_type: u16,
    isa: u16,
    version: u32,
    pub entry: u64,
    pub ph_offset: u64,
    pub sh_offset: u64,
    _flags: u32,
    header_size: u16,
    _ph_entry_size: u16,
    _ph_num: u16,
    _sh_entry_size: u16,
    _sh_num: u16,
    _sh_string_index: u16,
}

impl Elf64Header {
    /// Returns the class of this ELF64 header (32/64 bits).
    ///
    /// After being parsed in [`Elf64Header::new`], this header is guarenteed to be 64bits.
    pub fn class(&self) -> u8 {
        self.ident[4]
    }

    /// Returns the byte format of this ELF64 header (little/big endian).
    ///
    /// After being parsed in [`Elf64Header::new`], this header is guarenteed to be little endian.
    pub fn data(&self) -> u8 {
        self.ident[5]
    }

    /// Returns the version number in the identification part of the header.
    ///
    /// After being parsed in [`Elf64Header::new`], this version is guarenteed to be 1.
    pub fn ident_version(&self) -> u8 {
        self.ident[6]
    }

    /// Returns the magic values from this ELF64 header.
    pub fn magic(&self) -> [u8; 4] {
        self.ident[..4].try_into().unwrap()
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
        if header.magic() != Self::valid_magic() {
            return Err(ElfParseError::InvalidMagic);
        }
        // Ensure file is 64bit
        if header.class() != SIZE_64_BITS {
            return Err(ElfParseError::Not64Bit);
        }
        // Ensure little endian, as this only runs on x86_64
        if header.data() != LITTLE_ENDIAN {
            return Err(ElfParseError::NotLittleEndian);
        }
        // Ensure ELF is at current version
        if header.ident_version() != ELF_IDENT_VERSION && header.version == ELF_OLD_VERSION {
            return Err(ElfParseError::InvalidVersion);
        }
        // Ensure ABI is SystemV
        if header.os_abi() != SYSTEMV_ABI {
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

    /// Returns the ABI used in this ELF64 header.
    ///
    /// After being parsed in [`Elf64Header::new`], this header is guarenteed to use SystemV ABI.
    pub fn os_abi(&self) -> u8 {
        self.ident[7]
    }

    /// Returns the valid magic numbers for an ELF header.
    pub fn valid_magic() -> [u8; 4] {
        [0x7f, 0x45, 0x4c, 0x46]
    }
}
