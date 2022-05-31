use core::{num::Wrapping, ptr, slice, str};
use log::debug;
use uefi::{
    prelude::{Boot, SystemTable},
    table::cfg,
};

pub enum AcpiParseError {
    InvalidSignature,
    FailedChecksum,
}

const RSDP_SIZE: usize = 20;
const XSDP_SIZE: usize = 36;
const DESCRIPTION_HEADER_SIZE: usize = 36;

fn checksum(data: &[u8]) -> bool {
    let mut sum: Wrapping<u8> = Wrapping(0);
    for b in data {
        sum += b;
    }

    sum.0 == 0
}

#[repr(packed)]
pub struct RootDescriptionPointer {
    signature: [u8; 8],
    _checksum: u8,
    _oem_id: [u8; 6],
    revision: u8,
    rsdt_addr: u32,
}

impl RootDescriptionPointer {
    pub fn checksum(&self) -> bool {
        // TODO: Research safety
        let data = unsafe { slice::from_raw_parts((self as *const Self) as *const u8, RSDP_SIZE) };

        checksum(data)
    }

    pub fn signature(&self) -> Result<&str, ()> {
        str::from_utf8(&self.signature).map_err(|_| ())
    }

    pub fn valid_signature(&self) -> bool {
        if let Ok(signature) = self.signature() {
            signature == "RSD PTR "
        } else {
            false
        }
    }
}

#[repr(packed)]
pub struct ExtendedDescriptionPointer {
    _rsdp: RootDescriptionPointer,
    _length: u32,
    xsdt_addr: u64,
    _checksum: u8,
    _reserved: [u8; 3],
}

impl ExtendedDescriptionPointer {
    pub fn checksum(&self) -> bool {
        // TODO: Research safety
        let data = unsafe { slice::from_raw_parts((self as *const Self) as *const u8, XSDP_SIZE) };

        checksum(data)
    }
}

/// An ACPI table, including header and entries.
#[repr(packed)]
pub struct SystemDescriptionTable {
    header: DescriptionHeader,
    entries: [u8],
}

impl SystemDescriptionTable {
    pub fn checksum(&self) -> bool {
        let table_size = 36 + self.entries.len();
        // TODO: Research safety
        let data = unsafe {
            slice::from_raw_parts(
                &self.header as *const DescriptionHeader as *const u8,
                table_size,
            )
        };

        checksum(data)
    }

    pub unsafe fn from_uefi(st: &SystemTable<Boot>) -> &'static Self {
        let config_table = st.config_table();

        // Search for ACPI 2.0 table. Then search for ACPI 1.0 table if 2.0 is not found. Panic if neither
        // is found.
        let addr = if let Some(entry) = config_table.iter().find(|e| e.guid == cfg::ACPI2_GUID) {
            entry.address
        } else if let Some(entry) = config_table.iter().find(|e| e.guid == cfg::ACPI_GUID) {
            entry.address
        } else {
            panic!("Could not find ACPI table");
        } as *const u8;

        // Check if RSDP is actually RSDT/XSDT
        // Return RSDT/XSDT if it is valid
        if let Ok(table) = Self::try_parse(addr) {
            return table;
        }

        // Convert to RSDP struct
        // May not be valid
        let rsdp = (addr as *const RootDescriptionPointer).as_ref().unwrap();

        // Panic if signature is invalid
        if !rsdp.valid_signature() {
            panic!("Invalid RSDP signature");
        }

        // Panic if checksum failed
        if !rsdp.checksum() {
            panic!("RSDP checksum failed");
        }

        //------------------------------
        // RSDP is valid at this point
        //------------------------------

        // Return XSDT if available
        if rsdp.revision >= 2 {
            // Convert to XSDP struct
            // May not be valid
            let xsdp = (addr as *const ExtendedDescriptionPointer)
                .as_ref()
                .unwrap();

            // Panic if checksum failed
            if !xsdp.checksum() {
                panic!("XSDP checksum failed");
            }

            //------------------------------
            // XSDP is valid at this point
            //------------------------------

            // Return XSDT if it is valid
            if let Ok(xsdt) = Self::try_parse(xsdp.xsdt_addr as usize as *const u8) {
                return xsdt;
            } else {
                panic!("Invalid XSDT");
            }
        }

        // Return RSDT if it is valid
        if let Ok(rsdt) = Self::try_parse(rsdp.rsdt_addr as usize as *const u8) {
            rsdt
        } else {
            panic!("Invalid RSDT");
        }
    }

    pub unsafe fn try_parse(addr: *const u8) -> Result<&'static Self, AcpiParseError> {
        // Convert to header struct
        // It may or may not be valid
        let table_header = (addr as *const DescriptionHeader).as_ref().unwrap();

        // Check if signature is valid
        let signature = table_header
            .signature()
            .map_err(|_| AcpiParseError::InvalidSignature)?;

        // Return error if signature does not match
        if !matches!(signature, "RSDT" | "XSDT") {
            return Err(AcpiParseError::InvalidSignature);
        }

        // Get size of entire table
        let table_size = table_header.length as usize;

        // Convert to table struct
        // It may or may not be valid
        let table =
            ptr::from_raw_parts::<Self>(addr as *const (), table_size - DESCRIPTION_HEADER_SIZE)
                .as_ref()
                .unwrap();

        // Return error if checksum fails
        if !table.checksum() {
            return Err(AcpiParseError::FailedChecksum);
        }

        //-----------------------------------
        // RSDT/XSDT is valid at this point
        //-----------------------------------

        debug!(
            "Found {} of size {} at 0x{:x}",
            signature, table_size, addr as usize
        );

        Ok(table)
    }
}

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
    pub fn signature(&self) -> Result<&str, ()> {
        str::from_utf8(&self.signature).map_err(|_| ())
    }
}
