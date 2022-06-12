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
    pub entry: usize,
    pub ph_offset: usize,
    pub sh_offset: usize,
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
    fn valid_magic() -> &'static [u8; 4] {
        b"\x7fELF"
    }
}

impl Magic<4> for ElfHeader64 {
    fn magic(&self) -> &[u8; 4] {
        (&self.ident[..4]).try_into().unwrap()
    }
}

pub const ELF_SH_TYPE_SYMTAB: u32 = 2;
pub const ELF_SH_TYPE_STRTAB: u32 = 3;

/// An ELF64 section header.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ElfSectionHeader64 {
    pub name_idx: u32,
    pub section_type: u32,
    flags: usize,
    addr: usize,
    pub offset: usize,
    pub size: usize,
    link: u32,
    info: u32,
    addr_align: usize,
    pub entry_size: usize,
}

impl ElfSectionHeader64 {
    /// Returns the first section in `section_table` with a name matching `section_name` and a
    /// section type matching `section_type`.
    ///
    /// Returns `None` if no section with `section_name` exists.
    ///
    /// The `section_name` is found in the `str_table`.
    pub fn find_section<'a>(
        section_headers: &'a [Self],
        section_name: &[u8],
        section_type: u32,
        str_table: &[u8],
    ) -> Option<&'a Self> {
        section_headers.iter().find(|sh| {
            let name_idx = sh.name_idx as usize;
            if name_idx + section_name.len() > str_table.len() {
                return false;
            }
            &str_table[name_idx..name_idx + section_name.len()] == section_name
                && sh.section_type == section_type
        })
    }
}

pub const ELF_PH_TYPE_LOAD: u32 = 1;

/// An ELF64 program header.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ElfProgramHeader64 {
    pub program_type: u32,
    flags: u32,
    pub offset: usize,
    vaddr: usize,
    paddr: usize,
    pub file_size: usize,
    pub mem_size: usize,
    align: usize,
}

/// And ELF64 symbol
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct ElfSymbol64 {
    pub name_idx: u32,
    info: u8,
    other: u8,
    sh_index: u16,
    pub value: usize,
    size: u64,
}

impl ElfSymbol64 {
    /// Returns the first symbol in `symbol_table` with a name matching `symbol_name`.
    ///
    /// Returns `None` if no symbol with `symbol_name` exists.
    ///
    /// The `symbol_name` is found in the `symbol_str_table`.
    pub fn find_symbol<'a>(
        symbol_table: &'a [Self],
        symbol_name: &[u8],
        symbol_str_table: &[u8],
    ) -> Option<&'a Self> {
        symbol_table.iter().find(|symbol| {
            let name_idx = symbol.name_idx as usize;
            if name_idx + symbol_name.len() > symbol_str_table.len() {
                return false;
            }
            &symbol_str_table[name_idx..name_idx + symbol_name.len()] == symbol_name
        })
    }
}
