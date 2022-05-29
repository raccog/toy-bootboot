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
pub use fs::{open_dir, open_file, read_to_string, read_to_vec};
pub use initrd::Initrd;
pub use mmap::BootbootMMap;

use core::{
    ffi::c_void,
    slice,
    str::{self, FromStr},
};
use log::debug;
use uefi::{
    prelude::*,
    proto::{
        console::gop::{GraphicsOutput, Mode, ModeInfo},
        media::file::{Directory, File, FileAttribute, FileMode, RegularFile},
    },
    table::{
        boot::{BootServices, MemoryType},
        cfg,
    },
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

/// Uses UEFI Graphics Output Protocol to find an available graphics mode that closely matches the
/// `target_resolution`.
///
/// Returns the native mode if it matches the `target_resolution`.
///
/// If the mode that is closest to the `target_resolution` is not the native mode, then the GOP is
/// set to use the new mode. However, if this action fails then the native mode is returned.
///
/// # Panic
///
/// Panics if GOP cannot be located.
pub fn get_gop_info(bt: &BootServices, target_resolution: (usize, usize)) -> ModeInfo {
    // Get gop (graphics output protocol)
    let gop = unsafe {
        &mut *bt
            .locate_protocol::<GraphicsOutput>()
            .expect("Could not locate GOP")
            .get()
    };

    // Get GOP native mode
    let native_info = gop.current_mode_info();
    debug!(
        "Native mode: resolution={:?}, stride={}, format={:?}",
        native_info.resolution(),
        native_info.stride(),
        native_info.pixel_format()
    );

    // Return native mode if it matches the target resolution
    if native_info.resolution() == target_resolution {
        return native_info;
    }

    // If native resolution does not match target resolution:
    // Search each available mode to find one that is closest to matching the target resolution
    let mut selected_mode: Option<Mode> = None;
    for mode in gop.modes() {
        let info = mode.info();
        let resolution = info.resolution();

        // Compare to target resolution
        if resolution.0 >= target_resolution.0 && resolution.1 >= target_resolution.1 {
            if let Some(ref selected_mode_inner) = selected_mode {
                let selected_resolution = selected_mode_inner.info().resolution();
                // Compare to currently selected mode
                if resolution.0 < selected_resolution.0 && resolution.1 < selected_resolution.1 {
                    selected_mode = Some(mode);
                }
            } else {
                selected_mode = Some(mode);
            }
        }
    }

    // Set GOP mode to the selected mode
    // Returns native mode if this fails
    if let Some(ref selected_mode) = selected_mode {
        if gop.set_mode(selected_mode).is_err() {
            return native_info;
        }
    }

    // Return native mode if search failed
    selected_mode.map_or(native_info, |mode| *mode.info())
}

/// Uses UEFI Graphics Output Protocol to create a [`Framebuffer`] that most closely matches
/// `target_resolution`.
///
/// # Panic
///
/// Panics if GOP cannot be located.
pub fn get_framebuffer(bt: &BootServices, target_resolution: (usize, usize)) -> Framebuffer {
    // Get GOP mode
    let gop_info = get_gop_info(bt, target_resolution);
    debug!(
        "Selected mode: resolution={:?}, stride={}, format={:?}",
        gop_info.resolution(),
        gop_info.stride(),
        gop_info.pixel_format()
    );

    // Get gop (graphics output protocol)
    let gop = unsafe {
        &mut *bt
            .locate_protocol::<GraphicsOutput>()
            .expect("Could not locate GOP")
            .get()
    };
    let mut uefi_framebuffer = gop.frame_buffer();
    let ptr = uefi_framebuffer.as_mut_ptr() as usize as u64;
    let (width, height) = gop_info.resolution();
    let size = uefi_framebuffer.size() as u32;

    // Create Framebuffer from GOP info
    Framebuffer::new(ptr, size, width as u32, height as u32, gop_info.stride() as u32)
}

/// Returns a pointer to the RSDP.
///
/// If ACPI2.0 is supported, it will be preferred.
///
/// # Panic
///
/// Panics if no ACPI table could be found.
pub fn get_rsdp(st: &SystemTable<Boot>) -> *const c_void {
    let config_table = st.config_table();
    
    // Search for ACPI 2.0 table
    if let Some(entry) = config_table.iter().find(|e| matches!(e.guid, cfg::ACPI2_GUID)) {
        debug!("Found ACPI 2.0 table at: 0x{:x}", entry.address as usize);
        return entry.address;
    }

    // Search for ACPI 1.0 table
    if let Some(entry) = config_table.iter().find(|e| matches!(e.guid, cfg::ACPI_GUID)) {
        debug!("Found ACPI 1.0 table at: 0x{:x}", entry.address as usize);
        return entry.address;
    }

    panic!("Could not find ACPI table");
}

/// Returns a pointer to the RSDT/XSDT.
///
/// If ACPI2.0 is supported, it will be preferred.
///
/// # Panic
///
/// Panics if no ACPI table could be found.
pub fn get_acpi_table(st: &SystemTable<Boot>) -> *const c_void {
    // Get RSDP
    let rsdp = get_rsdp(st) as *const u8;
    let rsdp_signature = str::from_utf8(unsafe {
        slice::from_raw_parts(rsdp, 4)
    }).expect("Invalid ACPI signature");

    // Check if RSDP is actually RSDT/XSDT
    if matches!(rsdp_signature, "RSDT" | "XSDT") {
        debug!("Found {}", rsdp_signature);
        return rsdp as *const c_void;
    }

    // Panic if RSDP has invalid signature
    let rsdp_signature = str::from_utf8(unsafe {
        slice::from_raw_parts(rsdp, 8)
    }).expect("Invalid ACPI signature");
    if rsdp_signature != "RSD PTR " {
        panic!("Invalid RSDP signature");
    }

    // Get ACPI revision
    let revision = unsafe { *rsdp.add(15) };
    let rsdp_header_len = if revision == 0 {
        20
    } else {
        36
    };

    // RSDP header data
    let rsdp_header = unsafe {
        slice::from_raw_parts(rsdp, rsdp_header_len)
    };

    // Get pointer to RSDT/XSDT
    let table = if revision == 0 {
        u32::from_ne_bytes(rsdp_header[16..20].try_into().unwrap()) as usize
    } else {
        u64::from_ne_bytes(rsdp_header[24..32].try_into().unwrap()) as usize
    };

    // Check RSDT/XSDT signature
    let rsdt_signature = str::from_utf8(unsafe {
        slice::from_raw_parts(table as *const u8, 4)
    }).expect("Invalid RSDT/XSDT signature");
    if !matches!(rsdt_signature, "RSDT" | "XSDT") {
        panic!("Invalid RSDT/XSDT signature");
    }
    debug!("Found {}: 0x{:x}", rsdt_signature, table);

    table as *const c_void
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
    let acpi_table = get_acpi_table(&st);

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

    loop {}

    Status::SUCCESS
}
