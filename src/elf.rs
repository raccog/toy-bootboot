use core::{mem, slice};

use crate::utils::Magic;

/// An error resulting from parsing an ELF file.
#[derive(Copy, Clone, Debug)]
pub enum ElfParseError {
    InvalidAbi,
    InvalidFileType,
    InvalidIsa,
    InvalidMagic,
    InvalidOffset,
    InvalidSize,
    InvalidVersion,
    Not64Bit,
    NotLittleEndian,
    TooManyHeaders,
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
pub struct ElfHeader64 {
    ident: [u8; ELF_HEADER_NIDENT],
    file_type: u16,
    isa: u16,
    version: u32,
    pub entry: u64,
    pub ph_offset: u64,
    pub sh_offset: u64,
    _flags: u32,
    header_size: u16,
    pub ph_entry_size: u16,
    pub ph_num: u16,
    pub sh_entry_size: u16,
    pub sh_num: u16,
    pub sh_string_index: u16,
}

impl ElfHeader64 {
    /// Returns the class of this ELF64 header (32/64 bits).
    ///
    /// After being parsed in [`ElfHeader64::new`], this header is guarenteed to be 64bits.
    pub fn class(&self) -> u8 {
        self.ident[4]
    }

    /// Returns the byte format of this ELF64 header (little/big endian).
    ///
    /// After being parsed in [`ElfHeader64::new`], this header is guarenteed to be little endian.
    pub fn data(&self) -> u8 {
        self.ident[5]
    }

    /// Returns every section and program header in this ELF file.
    ///
    /// # Errors
    ///
    /// * `ElfParseError::TooManyHeaders`: Specified headers do not fit in `data`
    /// * `ElfParseError::InvalidSize`: Section/program header specified size does not match struct
    /// size
    /// * `ElfParseError::InvalidOffset`: Section/program header offset goes past `data` end
    pub fn get_headers(
        &self,
        data: &[u8],
    ) -> Result<(&[ElfSectionHeader64], &[ElfProgramHeader64]), ElfParseError> {
        // Get size of all headers
        let ph_size = self.ph_num as usize * self.ph_entry_size as usize;
        let sh_size = self.sh_num as usize * self.sh_entry_size as usize;
        let headers_size = mem::size_of::<ElfHeader64>() + ph_size + sh_size;
        if headers_size > data.len() {
            return Err(ElfParseError::TooManyHeaders);
        }
        // Ensure entry sizes match header sizes
        if self.ph_entry_size as usize != mem::size_of::<ElfProgramHeader64>()
            || self.sh_entry_size as usize != mem::size_of::<ElfSectionHeader64>()
        {
            return Err(ElfParseError::InvalidSize);
        }
        // Ensure offsets point to valid headers
        if self.sh_offset as usize >= data.len() || self.ph_offset as usize >= data.len() {
            return Err(ElfParseError::InvalidOffset);
        }
        // Get slices of section and program headers
        let section_headers = unsafe {
            slice::from_raw_parts(
                &data[self.sh_offset as usize] as *const u8 as *const ElfSectionHeader64,
                self.sh_num as usize,
            )
        };
        let program_headers = unsafe {
            slice::from_raw_parts(
                &data[self.ph_offset as usize] as *const u8 as *const ElfProgramHeader64,
                self.ph_num as usize,
            )
        };
        Ok((section_headers, program_headers))
    }

    /// Returns the version number in the identification part of the header.
    ///
    /// After being parsed in [`ElfHeader64::new`], this version is guarenteed to be 1.
    pub fn ident_version(&self) -> u8 {
        self.ident[6]
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
    /// After being parsed in [`ElfHeader64::new`], this header is guarenteed to use SystemV ABI.
    pub fn os_abi(&self) -> u8 {
        self.ident[7]
    }

    /// Returns the valid ELF magic numbers.
    fn valid_magic() -> [u8; 4] {
        // 0x7f "ELF"
        [0x7f, 0x45, 0x4c, 0x46]
    }
}

impl Magic<4> for ElfHeader64 {
    fn magic(&self) -> [u8; 4] {
        self.ident[..4].try_into().unwrap()
    }
}

/// An ELF64 section header.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ElfSectionHeader64 {
    name_idx: u32,
    section_type: u32,
    flags: u64,
    addr: u64,
    offset: u64,
    size: u64,
    link: u32,
    info: u32,
    addr_align: u64,
    entry_size: u64,
}

/// An ELF64 program header.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ElfProgramHeader64 {
    program_type: u32,
    flags: u32,
    offset: u64,
    vaddr: u64,
    paddr: u64,
    file_size: u64,
    mem_size: u64,
    align: u64,
}
