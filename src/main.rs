#![no_std]
#![no_main]
#![feature(abi_efiapi)]
#![feature(slice_take)]

mod bootboot;
mod environment;

use bootboot::{BootbootMMap};
use environment::get_environment;

use core::slice;
use log::{debug, error};
use uefi::{prelude::*, table::boot::MemoryType, CStr16};

#[entry]
fn main(image_handle: Handle, mut st: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut st).unwrap();

    // Log debug statements if built in debug mode
    if cfg!(debug_assertions) {
        log::set_max_level(log::LevelFilter::Debug);

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

    // Read BOOTBOOT/CONFIG to a page of memory
    let mut env: [u8; 4096] = [0; 4096];
    let size = get_environment(image_handle, &mut st, &mut env);
    if let Err(size) = size {
        error!("Config file of size {} could not fit in a page", size);
        return Status::BAD_BUFFER_SIZE;
    }
    let size = size.unwrap();

    // Convert environment to unicode for debug print
    if cfg!(debug_assertions) {
        let mut buf: [u16; 4096] = [0; 4096];
        for i in 0..4096 {
            buf[i] = env[i] as u16;
        }

        // Debug print environment
        let out = CStr16::from_u16_with_nul(&buf[0..size])
            .expect("Could not convert environment to CStr16");
        debug!("Environment:\n{}", out);
    }

    // Get memory map from UEFI
    let bt = st.boot_services();
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

    // Infinite loop to ensure UEFI is running this image
    loop {}

    Status::SUCCESS
}
