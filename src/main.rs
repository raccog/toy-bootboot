#![no_std]
#![no_main]
#![feature(abi_efiapi)]
#![feature(iter_advance_by)]
#![feature(ptr_metadata)]
#![feature(slice_take)]
//! This is a toy implementation of the BOOTBOOT protocol for x86_64 UEFI systems.
//!
//! It is a work in progress and an experimental project.
//! My main goal is to see what advantages and disadvantages there are in using Rust to make
//! freestanding programs; both in safety and in abstractions.
//!
//! If you want a non-experimental boot loader implementing the BOOTBOOT protocol, use the
//! [official reference implementation](https://gitlab.com/bztsrc/bootboot).
//!
//! # Bootloader Process
//!
//! This bootloader is structured so that the [`main`] function either loads a kernel and never
//! returns or it panics if an unrecoverable error is encountered.
//!
//! # Panics
//!
//! Unrecoverable errors cause the bootloader to panic, as the processor is forced to halt.
//!
//! All possible panics should generally be exposed in the `main.rs` source file. This will make it
//! easier to look for all points of possible failure. Every function outside `main.rs` should not
//! panic from unrecoverable errors.
//!
//! ## Panic Process
//!
//! Panics are different for release and debug modes.
//!
//! A panic in the release mode prints out an error code to the console and halt (not implemented
//! yet).
//!
//! A panic in the debug mode provides extra information about the error. Right now, the only
//! extra information is the source file and line of panic, but a stack trace might be helpful,
//! too.

extern crate alloc;

mod acpi;
mod environment;
mod framebuffer;
mod fs;
mod header;
mod initrd;
mod mmap;
mod smbios;

pub use acpi::SystemDescriptionTable;
pub use environment::{get_env, Environment};
pub use framebuffer::{get_framebuffer, Framebuffer};
pub use fs::{open_dir, open_file, read_to_string, read_to_vec};
pub use initrd::{get_initrd, Initrd};
pub use mmap::BootbootMMap;
pub use smbios::SmbiosEntryPoint;

use core::{slice, str};
use log::debug;
use uefi::{prelude::*, table::boot::MemoryType};

fn debug_info(st: &SystemTable<Boot>) {
    // Print firmware info
    let fw_revision = st.firmware_revision();
    let uefi_revision = st.uefi_revision();

    debug!("UEFI firmware information:");
    debug!("Vendor = {}", st.firmware_vendor());
    debug!(
        "Firmware Revision = {}.{}",
        fw_revision.major(),
        fw_revision.minor()
    );
    debug!(
        "UEFI Revision = {}.{}",
        uefi_revision.major(),
        uefi_revision.minor()
    );
}

/*
fn get_smbios_table(config_table: &[ConfigTableEntry]) -> &'static [u8] {
    let addr = if let Some(entry) = config_table.iter().find(|e| matches!(e.guid, cfg::SMBIOS_GUID)) {
        debug!("Found SMBIOS");
        entry
    } else if let Some(entry) = config_table.iter().find(|e| matches!(e.guid, cfg::SMBIOS_GUID)) {
        debug!("Found SMBIOS3");
    } else {
        painic!("Could not find SMBIOS");
    } as *const u8;


}
*/

#[entry]
pub fn main(image_handle: Handle, mut st: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut st).unwrap();

    // Log debug statements if built in debug mode
    if cfg!(debug_assertions) {
        log::set_max_level(log::LevelFilter::Debug);

        debug_info(&st);
    }

    let bt = st.boot_services();

    // Get root directory of ESP
    // Panic if failed
    const ESP_ERR: &str = "No boot partition";
    let fs = bt.get_image_file_system(image_handle).expect(ESP_ERR);
    let fs = unsafe { &mut *fs.interface.get() };
    let mut root = fs.open_volume().expect(ESP_ERR);

    // Check for BOOTBOOT directory
    // Panic if not found
    let mut bootdir = open_dir(&mut root, "BOOTBOOT").expect(ESP_ERR);

    //------------------------
    // Step 1:
    // Read initrd file to memory
    //------------------------

    let initrd = get_initrd(&mut bootdir);
    debug!("Found initrd of size: {} KiB", initrd.len() / 1024);

    //-----------------------------
    // Step 2:
    // Read/Create BOOTBOOT environment
    //-----------------------------

    let env = get_env(&mut bootdir, &initrd);
    debug!("Kernel name: {}", env.kernel);
    debug!("SMP: {}", !env.no_smp);
    debug!("Target resolution: {:?}", env.screen);

    //----------------------
    // Step 4:
    // Initialize Hardware
    //----------------------

    // Get linear framebuffer
    let framebuffer = get_framebuffer(bt, env.screen);
    debug!("Framebuffer: {:?}", framebuffer);

    // Get ACPI table
    let _acpi_table = unsafe { SystemDescriptionTable::from_uefi(&st) };

    // Get SMBIOS
    //let smbios_table = get_smbios_table(st.config_table());
    let _smbios_table = unsafe { SmbiosEntryPoint::from_uefi(&st) };

    // Get memory map from UEFI
    let mmap_size = bt.memory_map_size();
    let entry_size = mmap_size.entry_size;
    let mmap_size = mmap_size.map_size + 2 * entry_size;
    let buffer = bt
        .allocate_pool(MemoryType::LOADER_DATA, mmap_size)
        .expect("Could not allocate pool for memory map");
    let buffer = unsafe { slice::from_raw_parts_mut(buffer, mmap_size) };
    let (_key, desc_iter) = bt
        .memory_map(buffer)
        .expect("Failed to get UEFI memory map");

    // Convert UEFI memory map to BOOTBOOT memory map
    let mmap = BootbootMMap::from_uefi_mmap(desc_iter);
    debug!("{}", mmap);

    panic!("Bootloader done (this will be removed when os loading is implemented)");
}
