use log::debug;
use uefi::table::cfg::{self, ConfigTableEntry};

use crate::utils::{Checksum, Magic, ParseError};

/// SMBIOS entry point struct.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct SmbiosEntryPoint {
    anchor: [u8; 4],
    _entry_point_checksum: u8,
    entry_point_length: u8,
    _version_major: u8,
    _version_minor: u8,
    _max_struct_size: u16,
    _entry_point_revision: u8,
    _formatted_area: [u8; 5],
    _intermediate_anchor: [u8; 5],
    _intermediate_checksum: u8,
    _table_length: u16,
    _table_address: u32,
    _num_structs: u16,
    _bcd_revision: u8,
}

impl Magic<4> for SmbiosEntryPoint {
    fn magic(&self) -> &[u8; 4] {
        &self.anchor
    }
}

impl Checksum for SmbiosEntryPoint {}

impl SmbiosEntryPoint {
    /// Parses the UEFI config tables to get the SMBIOS table.
    ///
    /// # Errors
    ///
    /// * `ParseError::NoTable`: SMBIOS table cannot be found
    /// * `ParseError::FailedChecksum`: SMBIOS checksum failed
    /// * `ParseError::InvalidSignature`: SMBIOS signature is invalid
    /// * `ParseError::InvalidPointer`: A null pointer was found during parse
    pub fn from_uefi_config_table(
        config_table: &[ConfigTableEntry],
    ) -> Result<&SmbiosEntryPoint, ParseError> {
        // Search config table for SMBIOS
        let smbios_entry = config_table
            .iter()
            .find(|e| e.guid == cfg::SMBIOS_GUID)
            .ok_or(ParseError::NoTable)?;
        let smbios_addr = smbios_entry.address;

        // Convert to SMBIOS struct
        // May not be valid
        let smbios = unsafe {
            (smbios_addr as *const Self)
                .as_ref()
                .ok_or(ParseError::InvalidPointer)?
        };

        // Return error if signature is invalid
        if smbios.magic() != Self::valid_magic() {
            return Err(ParseError::InvalidSignature);
        }

        // Return error if checksum failed
        if !smbios.checksum_valid() {
            return Err(ParseError::FailedChecksum);
        }

        //--------------------------------
        // SMBIOS is valid at this point
        //--------------------------------

        debug!(
            "Found SMBIOS of size 0x{:x} at 0x{:x}",
            smbios.entry_point_length, smbios_entry.address as usize
        );
        Ok(smbios)
    }

    pub fn valid_magic() -> &'static [u8; 4] {
        b"_SM_"
    }
}
