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


/// Returns true if the sum of every byte ends in 0x00.
///
/// In other words, this returns true if `sum % 0x100 == 0`.
///
/// This is used often in ACPI tables.
fn zerosum_checksum(data: &[u8]) -> bool {
    // Wrapping is used because only the last byte matters.
    let mut sum = Wrapping(0);

    for b in data {
        sum += *b;
    }

    sum.0 == 0
}

fn get_rsdt(addr: *const u8) -> &'static [u8] {
    // Convert RSDT/XSDT pointer to slice
    let rsdt = unsafe {
        slice::from_raw_parts(addr, 36)
    };

    // Check RSDT/XSDT signature
    const RSDT_SIG_ERR: &str = "Invalid RSDT/XSDT signature";
    let rsdt_signature = str::from_utf8(&rsdt[..4])
        .expect(RSDT_SIG_ERR);
    if !matches!(rsdt_signature, "RSDT" | "XSDT") {
        panic!("{}", RSDT_SIG_ERR);
    }

    // Get number of entries in RSDT/XSDT
    let rsdt_size = u32::from_ne_bytes(rsdt[4..8].try_into().unwrap()) as usize;

    // Expand RSDT/XSDT slice to contain all entries
    let rsdt = unsafe {
        slice::from_raw_parts(addr, rsdt_size)
    };

    // Check RSDT/XSDT checksum
    if !zerosum_checksum(rsdt) {
        panic!("{} checksum failed", rsdt_signature);
    }

    debug!("Found {} of size {} at 0x{:x}", rsdt_signature, rsdt_size, addr as usize);

    rsdt
}

/// Returns a pointer to the RSDT/XSDT.
///
/// If ACPI2.0 is supported, it will be preferred.
///
/// # Panic
///
/// Panics if:
///
/// * No ACPI table could be found
/// * Checksum fails in RSDP or RSDT/XSDT
pub fn get_acpi_table(st: &SystemTable<Boot>) -> &'static [u8] {
    let config_table = st.config_table();
    
    // Search for ACPI 2.0 table. Search for ACPI 1.0 table if 2.0 is not found. Panic if neither
    // is found.
    let addr = if let Some(entry) = config_table.iter().find(|e| matches!(e.guid, cfg::ACPI2_GUID)) {
        entry.address
    } else if let Some(entry) = config_table.iter().find(|e| matches!(e.guid, cfg::ACPI_GUID)) {
        entry.address
    } else {
        panic!("Could not find ACPI table");
    } as *const u8;

    // Check for RSDT/XSDT signature (first 4 bytes)
    let rsdp_signature = str::from_utf8(unsafe {
        slice::from_raw_parts(addr, 4)
    }).expect("Invalid ACPI signature");

    // Check if RSDP is actually RSDT/XSDT
    if matches!(rsdp_signature, "RSDT" | "XSDT") {
        return get_rsdt(addr);
    }

    // Get ACPI revision
    let rsdp = unsafe {
        slice::from_raw_parts(addr, 20)
    };
    let revision = rsdp[15];

    // Panic if RSDP has invalid signature
    const RSDP_SIG_ERR: &str = "Invalid RSDP signature";
    let rsdp_signature = str::from_utf8(&rsdp[..8])
        .expect(RSDP_SIG_ERR);
    if rsdp_signature != "RSD PTR " {
        panic!("{}", RSDP_SIG_ERR);
    }

    // Panic if checksum in RSDP fails
    if !zerosum_checksum(rsdp) {
        panic!("ACPI 1.0 checksum in RSDP failed");
    }

    debug!("Found RSDP revision {} at: 0x{:x}", revision, addr as usize);

    // Expand to XSDT if available
    let rsdp = if revision >= 2 {
        unsafe { slice::from_raw_parts(addr, 36) }
    } else {
        rsdp
    };

    // If XSDT, panic if second checksum fails
    if revision >= 2 && !zerosum_checksum(rsdp) {
        panic!("ACPI 2.0 checksum in RSDP failed");
    }

    // Get pointer to RSDT/XSDT
    let addr = if revision >= 2 {
        u64::from_ne_bytes(rsdp[24..32].try_into().unwrap()) as usize
    } else {
        u32::from_ne_bytes(rsdp[16..20].try_into().unwrap()) as usize
    } as *const u8;


    get_rsdt(addr)
}
