use core::{mem, ptr, slice, str};
use log::debug;
use uefi::table::cfg::{self, ConfigTableEntry};

use crate::utils::{self, Checksum, ParseError};

/// The RSDP struct that points to ACPI tables.
#[repr(packed)]
#[derive(Copy, Clone, Debug)]
pub struct RootDescriptionPointer {
    signature: [u8; 8],
    _checksum: u8,
    _oem_id: [u8; 6],
    revision: u8,
    rsdt_addr: u32,
}

impl RootDescriptionPointer {
    /// Returns the 8 byte signature of the RSDP.
    ///
    /// # Errors
    ///
    /// Returns an error if the signature is not valid UTF-8.
    pub fn signature(&self) -> Result<&str, ParseError> {
        str::from_utf8(&self.signature).map_err(|_| ParseError::InvalidSignature)
    }

    /// Returns true if the RSDP signature is valid.
    pub fn valid_signature(&self) -> bool {
        if let Ok(signature) = self.signature() {
            signature == "RSD PTR "
        } else {
            false
        }
    }
}

impl Checksum for RootDescriptionPointer {}

/// The RSDP struct that points to ACPI table with a valid XSDT.
///
/// This struct is used when the ACPI revision >= 2.
#[repr(packed)]
#[derive(Copy, Clone, Debug)]
pub struct ExtendedDescriptionPointer {
    _rsdp: RootDescriptionPointer,
    _length: u32,
    xsdt_addr: u64,
    _checksum: u8,
    _reserved: [u8; 3],
}

impl Checksum for ExtendedDescriptionPointer {}

/// An ACPI table (XSDT or RSDT), including header and entries.
///
/// As this table has a variable number of entries, it is not `Sized`.
#[repr(packed)]
pub struct AcpiSystemDescriptionTable {
    header: DescriptionHeader,
    entries: [u8],
}

impl AcpiSystemDescriptionTable {
    /// Returns true if the checksum is valid.
    ///
    /// This is a separate checksum from [`Checksum`] because `AcpiSystemDescriptionTable` is `?Sized`.
    pub fn checksum_valid(&self) -> bool {
        let table_size = 36 + self.entries.len();
        let data = unsafe {
            slice::from_raw_parts(
                &self.header as *const DescriptionHeader as *const u8,
                table_size,
            )
        };

        utils::checksum(data) == 0
    }

    /// Parses the UEFI config tables to find the XSDT or RSDT (XSDT is preferred).
    ///
    /// # Errors
    ///
    /// * `ParseError::NoTable`: ACPI table cannot be found
    /// * `ParseError::FailedChecksum`: RSDP or XSDT/RSDT checksum failed
    /// * `ParseError::InvalidSignature`: RSDP or XSDT/RSDT signature is invalid
    /// * `ParseError::InvalidPointer`: A null pointer was found during parse
    pub fn from_uefi_config_table(config_table: &[ConfigTableEntry]) -> Result<&Self, ParseError> {
        // Get RSDP from UEFI config table
        let acpi_table = get_acpi_table(config_table)?;
        let addr = acpi_table.address as *const ();

        // Convert to RSDP struct
        // May not be valid
        // Return error if RSDP is null pointer
        let rsdp = unsafe {
            (addr as *const RootDescriptionPointer)
                .as_ref()
                .ok_or(ParseError::InvalidPointer)?
        };

        // Return error if signature is invalid
        if !rsdp.valid_signature() {
            return Err(ParseError::InvalidSignature);
        }

        // Panic if checksum failed
        if !rsdp.checksum_valid() {
            return Err(ParseError::FailedChecksum);
        }

        //------------------------------
        // RSDP is valid at this point
        //------------------------------

        // Get address of either RSDT or XSDT
        let table_addr = if rsdp.revision >= 2 {
            // Convert to XSDP struct
            // May not be valid
            let xsdp = unsafe {
                (addr as *const ExtendedDescriptionPointer)
                    .as_ref()
                    .unwrap()
            };

            // Return error if checksum failed
            if !xsdp.checksum_valid() {
                return Err(ParseError::FailedChecksum);
            }

            //------------------------------
            // XSDP is valid at this point
            //------------------------------

            // Get XSDT address
            xsdp.xsdt_addr
        } else {
            rsdp.rsdt_addr as u64
        };

        // Convert to header struct
        // It may or may not be valid
        // Return error if null pointer
        let table_header = unsafe {
            (table_addr as *const DescriptionHeader)
                .as_ref()
                .ok_or(ParseError::InvalidPointer)?
        };

        // Return error if signature is valid
        let signature = table_header
            .signature()
            .map_err(|_| ParseError::InvalidSignature)?;

        // Return error if signature does not match
        if !matches!(signature, "RSDT" | "XSDT") {
            return Err(ParseError::InvalidSignature);
        }

        // Get size of entire table
        let table_size = table_header.length as usize;

        // Convert to table struct
        // It may or may not be valid
        let table = unsafe {
            ptr::from_raw_parts::<Self>(
                table_addr as *const (),
                table_size - mem::size_of::<DescriptionHeader>(),
            )
            .as_ref()
            .unwrap()
        };

        // Return error if checksum fails
        if !table.checksum_valid() {
            return Err(ParseError::FailedChecksum);
        }

        //-----------------------------------
        // RSDT/XSDT is valid at this point
        //-----------------------------------

        debug!(
            "Found {} of size 0x{:x} at 0x{:x}",
            signature, table_size, addr as usize
        );

        Ok(table)
    }
}

/// A header for an ACPI table.
#[repr(packed)]
pub struct DescriptionHeader {
    signature: [u8; 4],
    length: u32,
    _revision: u8,
    _checksum: u8,
    _oem_id: [u8; 6],
    _oem_table_id: u64,
    _oem_revision: u32,
    _creator_id: u32,
    _creator_revision: u32,
}

impl DescriptionHeader {
    /// Returns the 4 byte signature of the ACPI header.
    ///
    /// # Errors
    ///
    /// Returns an error if the signature is not valid UTF-8.
    pub fn signature(&self) -> Result<&str, ()> {
        str::from_utf8(&self.signature).map_err(|_| ())
    }
}

fn get_acpi_table(config_table: &[ConfigTableEntry]) -> Result<&ConfigTableEntry, ParseError> {
    // Search for ACPI 2.0 table.
    if let Some(entry) = config_table.iter().find(|e| e.guid == cfg::ACPI2_GUID) {
        debug!("Found ACPI 2.0 table");
        return Ok(entry);
    }

    // Search for ACPI 1.0 table. Return None if not found
    if let Some(entry) = config_table.iter().find(|e| e.guid == cfg::ACPI_GUID) {
        debug!("Found ACPI 1.0 table");
        return Ok(entry);
    }

    Err(ParseError::NoTable)
}
