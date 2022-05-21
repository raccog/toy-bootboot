#![no_std]
#![no_main]
#![feature(abi_efiapi)]
#![feature(iter_advance_by)]
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

mod environment;
mod framebuffer;
mod fs;
mod header;
mod initrd;
mod mmap;

pub use environment::Environment;
pub use framebuffer::Framebuffer;
pub use fs::{
    open_dir, open_file, read_to_string, read_to_vec,
};
pub use initrd::Initrd;
pub use mmap::BootbootMMap;

use core::{
    slice,
    str::{self, FromStr},
};
use log::debug;
use uefi::{
    prelude::*,
    proto::media::file::{Directory, File, FileAttribute, FileMode, RegularFile},
    table::boot::MemoryType,
    Result as UefiResult,
};

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

fn get_initrd_file(bootdir: &mut Directory) -> UefiResult<RegularFile> {
    // Try to open BOOTBOOT/INITRD
    let initrd_file = open_file(bootdir, "INITRD", FileMode::Read, FileAttribute::empty());
    if initrd_file.is_ok() {
        return initrd_file;
    }

    // Try to open BOOTBOOT/X86_64
    open_file(bootdir, "X86_64", FileMode::Read, FileAttribute::empty())
}

/// Reads initrd file from boot partition.
///
/// The default file path is `BOOTBOOT/INITRD`.
///
/// # Panics
///
/// If the initrd file cannot be found.
pub fn get_initrd(bootdir: &mut Directory) -> Initrd {
    // Initrd file
    const INITRD_ERR: &str = "Initrd not found";
    let mut initrd_file = get_initrd_file(bootdir).expect(INITRD_ERR);

    // Read initrd
    let initrd = read_to_vec(&mut initrd_file).expect(INITRD_ERR);
    let initrd = Initrd::new(initrd);

    // Close initrd file
    initrd_file.close();

    initrd
}

/// Returns the BOOTBOOT environment to pass to the kernel.
///
/// The following steps are run until a valid environment is returned:
///
/// 1. Try to read `BOOTBOOT/CONFIG` from boot partition and parse environment.
/// 2. Try to read `sys/config` from `initrd` and parse environment.
/// 3. If neither file contains a valid environment, return a default environment.
pub fn get_env(bootdir: &mut Directory, initrd: &Initrd) -> Environment {
    // Try to open BOOTBOOT/CONFIG
    if let Ok(mut env_file) = open_file(bootdir, "CONFIG", FileMode::Read, FileAttribute::empty()) {
        // Read config file to string
        if let Ok(env_raw) = read_to_string(&mut env_file) {
            // Parse environment
            if let Ok(env) = Environment::from_str(&env_raw) {
                debug!("Found BOOTBOOT/CONFIG in boot partition");
                return env;
            }
        }

        // CONFIG file close
        env_file.close();
    }

    // Try to open sys/config in initrd
    if let Some(env_raw) = initrd.read_file("sys/config") {
        // Convert config file to string
        if let Ok(env_raw) = str::from_utf8(env_raw) {
            if let Ok(env) = Environment::from_str(env_raw) {
                debug!("Found sys/config in initrd");
                return env;
            }
        }
    }

    // Return default environment
    debug!("Using default environment");
    Environment::default()
}

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
    debug!("Screen size: {} x {}", env.screen.0, env.screen.1);
    debug!("Kernel name: {}", env.kernel);
    debug!("SMP: {}", !env.no_smp);

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

    loop {}

    Status::SUCCESS
}
