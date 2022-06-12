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
mod elf;
mod environment;
mod framebuffer;
mod fs;
mod header;
mod initrd;
mod mmap;
mod smbios;
mod time;
mod utils;

pub use acpi::AcpiSystemDescriptionTable;
pub use elf::{
    ElfHeader64, ElfParseError, ElfProgramHeader64, ElfSectionHeader64, ElfSymbol64,
    ELF_PH_TYPE_LOAD, ELF_SH_TYPE_STRTAB, ELF_SH_TYPE_SYMTAB,
};
pub use environment::Environment;
pub use framebuffer::Framebuffer;
pub use fs::{open_dir, open_file, read_to_string, read_to_vec};
pub use initrd::Initrd;
pub use mmap::BootbootMMap;
pub use smbios::SmbiosEntryPoint;

use alloc::{vec, vec::Vec};
use core::{mem, slice, str};
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

/// Parses `kernel` to load executable, symbol information, and the program header for the loaded
/// region.
///
/// Returns a tuple that includes the loaded executable and all symbols found.
///
/// # Panic
///
/// Panics if `kernel` is invalid AND also gets in the way of loading an executable. If there are
/// invalid parts of the file that do not contribute to loading the executable, no panic will
/// occur.
fn load_elf<'a>(
    elf_header: &'a ElfHeader64,
    kernel: &[u8],
) -> (
    Vec<u8>,
    [Option<&'a ElfSymbol64>; 4],
    &'a ElfProgramHeader64,
) {
    // Get section and program headers
    let (section_headers, program_headers) = elf_header
        .get_headers(kernel)
        .unwrap_or_else(|e| panic!("Kernel: Error while parsing ELF file headers: {:?}", e));

    // Get first program header with LOAD type
    let ph_load = program_headers
        .iter()
        .find(|ph| ph.program_type == ELF_PH_TYPE_LOAD)
        .expect("Kernel: No program header of LOAD type");
    // Ensure program header is valid
    if ph_load.offset + ph_load.file_size > kernel.len() {
        panic!(
            "Kernel: File size {} bytes with offset 0x{:x} is too small to load executable of size {} bytes",
            kernel.len(),
            ph_load.offset,
            ph_load.file_size
        );
    }
    if ph_load.file_size > ph_load.mem_size {
        panic!("Kernel: Size of executable file should not be larger than size in memory");
    }
    if elf_header.sh_string_index as usize >= section_headers.len() {
        panic!("Kernel: String table has an invalid section index");
    }

    // Get info from program header
    let kernel_load = &kernel[ph_load.offset..ph_load.offset + ph_load.file_size];

    // Get string table for section names
    let str_table_header = section_headers[elf_header.sh_string_index as usize];
    if str_table_header.offset + str_table_header.size > kernel.len() {
        panic!("Kernel: String table has invalid size or offset");
    }
    let str_table =
        &kernel[str_table_header.offset..str_table_header.offset + str_table_header.size];

    // Get symbol table by checking for ".symtab" in string table
    let symbol_name = b".symtab";
    let symbol_header = ElfSectionHeader64::find_section(
        section_headers,
        &symbol_name[..],
        ELF_SH_TYPE_SYMTAB,
        &str_table,
    )
    .expect("Kernel: Could not find valid symbol table header");
    if symbol_header.entry_size != mem::size_of::<ElfSymbol64>() {
        panic!("Kernel: Symbol table has invalid entry size");
    }
    if symbol_header.offset + symbol_header.size > kernel.len() {
        panic!("Kernel: Symbol table does not fit");
    }
    if symbol_header.size % symbol_header.entry_size != 0
        || symbol_header.size < symbol_header.entry_size
    {
        panic!("Kernel: Symbol table has invalid size");
    }
    let symbol_entries = symbol_header.size / symbol_header.entry_size;
    let symbol_table = unsafe {
        slice::from_raw_parts(
            &kernel[symbol_header.offset] as *const u8 as *const ElfSymbol64,
            symbol_entries,
        )
    };

    // Get symbol string table by checking for ".strtab" in string table
    let symbol_str_name = b".strtab";
    let symbol_str_header = ElfSectionHeader64::find_section(
        section_headers,
        &symbol_str_name[..],
        ELF_SH_TYPE_STRTAB,
        &str_table,
    )
    .expect("Kernel: Could not find valid symbol string table header");
    if symbol_str_header.offset + symbol_str_header.size > kernel.len() {
        panic!("Kernel: Symbol string table has invalid size or offset");
    }
    let symbol_str_table =
        &kernel[symbol_str_header.offset..symbol_str_header.offset + symbol_str_header.size];

    // Find special symbols
    let bootboot_symbol_name = b"bootboot";
    let bootboot_symbol =
        ElfSymbol64::find_symbol(symbol_table, &bootboot_symbol_name[..], symbol_str_table);
    let env_symbol_name = b"environment";
    let env_symbol = ElfSymbol64::find_symbol(symbol_table, &env_symbol_name[..], symbol_str_table);
    let fb_symbol_name = b"fb";
    let fb_symbol = ElfSymbol64::find_symbol(symbol_table, &fb_symbol_name[..], symbol_str_table);
    let initstack_symbol_name = b"initstack";
    let initstack_symbol =
        ElfSymbol64::find_symbol(symbol_table, &initstack_symbol_name[..], symbol_str_table);

    debug!(
        "Found ELF executable of size: {} KiB",
        kernel_load.len() / 1024
    );
    if let Some(bootboot) = bootboot_symbol {
        debug!("Symbol BOOTBOOT: 0x{:x}", bootboot.value);
    }
    if let Some(env) = env_symbol {
        debug!("Symbol ENVIRONMENT: 0x{:x}", env.value);
    }
    if let Some(fb) = fb_symbol {
        debug!("Symbol FRAMEBUFFER: 0x{:x}", fb.value);
    }
    if let Some(initstack) = initstack_symbol {
        debug!("Symbol INITSTACK: 0x{:x}", initstack.value);
    }

    // Ensure kernel is valid executable

    // Allocate space for kernel
    let mut loaded_kernel = vec![0; ph_load.mem_size];
    loaded_kernel[..ph_load.file_size].copy_from_slice(kernel_load);

    let all_symbols = [bootboot_symbol, env_symbol, fb_symbol, initstack_symbol];
    (loaded_kernel, all_symbols, ph_load)
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
    const ESP_ERR: &str = "No boot partition";
    let fs = bt.get_image_file_system(image_handle).expect(ESP_ERR);
    let fs = unsafe { &mut *fs.interface.get() };
    let mut root = fs.open_volume().expect(ESP_ERR);

    // Check for BOOTBOOT directory
    let mut bootdir = open_dir(&mut root, "BOOTBOOT").expect(ESP_ERR);

    // Read initrd file into memory
    let initrd = Initrd::from_disk(&mut bootdir).expect("Could not read initrd from disk");
    debug!("Found initrd of size: {} KiB", initrd.size() / 1024);

    let env = Environment::get_env(&mut bootdir, &initrd);
    debug!("Kernel name: {}", env.kernel);
    debug!("SMP: {}", !env.no_smp);
    debug!("Target resolution: {:?}", env.screen);

    // Get linear framebuffer
    let framebuffer =
        Framebuffer::from_boot_services(bt, env.screen).expect("Could not get framebuffer");
    debug!("Framebuffer: {:?}", framebuffer);

    // Get ACPI table
    let _acpi_table = AcpiSystemDescriptionTable::from_uefi_config_table(st.config_table());

    // Get SMBIOS
    let _smbios_table = SmbiosEntryPoint::from_uefi_config_table(st.config_table());

    // Get time
    if let Ok(time) = time::get_time(&st) {
        debug!("Got time: {:?}", time);
    }

    // Get kernel ELF file
    // Panic if not found
    let kernel = initrd
        .read_file(&env.kernel)
        .unwrap_or_else(|| panic!("Could not read kernel at file: {}", env.kernel));
    // Panic if too small
    if kernel.len() < mem::size_of::<ElfHeader64>() {
        panic!("Kernel of size {} bytes is too small", kernel.len());
    }
    debug!(
        "Found kernel at file {} of size {} KiB",
        env.kernel,
        kernel.len() / 1024
    );

    // Get ELF64 header
    let elf_header = ElfHeader64::new(kernel[..mem::size_of::<ElfHeader64>()].try_into().unwrap())
        .unwrap_or_else(|e| panic!("Error while parsing Elf header: {:?}", e));

    // Load kernel executable
    let (_loaded_kernel, _all_symbols, _ph_load) = load_elf(&elf_header, kernel);

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
