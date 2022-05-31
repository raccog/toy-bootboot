use core::{
    num::Wrapping,
    slice,
    str,
};
use log::debug;
use uefi::{
    prelude::{Boot, SystemTable},
    table::cfg,
};

#[repr(packed)]
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

impl SmbiosEntryPoint {
    pub fn checksum(&self) -> bool {
        let length = self.entry_point_length;
        let mut sum: Wrapping<u8> = Wrapping(0);
        // TODO: Research safety
        let data = unsafe { slice::from_raw_parts(self as *const Self as *const u8, length as usize) };
        for b in data {
            sum += b;
        }

        sum.0 == 0
    }

    pub unsafe fn from_uefi(st: &SystemTable<Boot>) -> &'static SmbiosEntryPoint {
        let config_table = st.config_table();

        // Search config table for SMBIOS
        let smbios_entry = config_table.iter().find(|e| e.guid == cfg::SMBIOS_GUID)
            .unwrap_or_else(|| panic!("Could not find SMBIOS in config table"));
        let smbios_addr = smbios_entry.address;

        // Convert to SMBIOS struct
        // May not be valid
        let smbios = &*(smbios_addr as *const Self);

        // Panic if signature is invalid
        if !smbios.valid_signature() {
            panic!("Invalid SMBIOS anchor");
        }

        // Panic if checksum failed
        if !smbios.checksum() {
            panic!("SMBIOS checksum failed");
        }

        //--------------------------------
        // SMBIOS is valid at this point
        //--------------------------------

        debug!("Found SMBIOS of size 0x{:x} at 0x{:x}", smbios.entry_point_length, smbios_entry.address as usize);
        smbios
    }

    pub fn signature(&self) -> Result<&str, ()> {
        str::from_utf8(&self.anchor)
            .map_err(|_| ())
    }

    pub fn valid_signature(&self) -> bool {
        if let Ok(signature) = self.signature() {
            signature == "_SM_"
        } else {
            false
        }
    }
}

