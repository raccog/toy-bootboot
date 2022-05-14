#![no_std]
#![no_main]
#![feature(abi_efiapi)]
#![feature(iter_advance_by)]
#![feature(slice_take)]

extern crate alloc;

mod environment;
mod framebuffer;
mod fs;
mod header;
mod initrd;
mod mmap;

pub use environment::Environment;
pub use framebuffer::Framebuffer;
pub use initrd::Initrd;
pub use mmap::BootbootMMap;
pub use fs::{open_dir, open_dir_or_panic, open_file, open_file_or_panic, read_to_string, read_to_vec};

use core::{
    slice,
    str::FromStr
};
use log::{debug, error};
use uefi::{
    prelude::*,
    proto::media::file::{Directory, File, FileAttribute, FileMode, RegularFile},
    table::boot::MemoryType,
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

fn get_initrd(dir: &mut Directory) -> RegularFile {
    // Check 'BOOTBOOT/INITRD'
    if let Ok(file) = open_file(dir, "INITRD", FileMode::Read, FileAttribute::empty()) {
        return file;
    }

    // Check 'BOOTBOOT/X86_64'
    open_file_or_panic(dir, "X86_64", FileMode::Read, FileAttribute::empty())
}

#[entry]
fn main(image_handle: Handle, mut st: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut st).unwrap();

    // Log debug statements if built in debug mode
    if cfg!(debug_assertions) {
        log::set_max_level(log::LevelFilter::Debug);

        debug_info(&st);
    }

    let bt = st.boot_services();

    // Get root directory of ESP
    // Panic if failed
    let fs = bt
        .get_image_file_system(image_handle)
        .expect("Failed to get EFI boot partition");
    let fs = unsafe { &mut *fs.interface.get() };
    let mut root = fs
        .open_volume()
        .expect("Failed to open root directory on boot partition");

    // Check for BOOTBOOT directory
    // Panic if not found
    let mut dir = open_dir_or_panic(&mut root, "BOOTBOOT");

    //------------------------
    // Step 1:
    // Read initrd file to memory
    //------------------------

    // INITRD file
    let mut initrd_file = get_initrd(&mut dir);

    // Read initrd
    let initrd = Initrd::new(read_to_vec(&mut initrd_file).expect("Could not read initrd file"));

    initrd_file.close();

    let file = initrd.read_file("test.txt");

    //------------------------
    // Step 2:
    // Search for config file
    //------------------------

    // CONFIG file
    let mut file = open_file_or_panic(&mut dir, "CONFIG", FileMode::Read, FileAttribute::empty());

    //-----------------------------
    // Step 3:
    // Create BOOTBOOT environment
    //-----------------------------

    // Read config file to vector
    let config_raw = read_to_string(&mut file).expect("Could not read BOOTBOOT/CONFIG");

    // CONFIG file close
    file.close();

    let env = Environment::from_str(&config_raw)
        .expect("Could not parse config file");
    debug!("Screen size: {} x {}", env.screen.0, env.screen.1);
    debug!("Kernel name: {}", env.kernel);
    debug!("SMP: {}", !env.no_smp);

    //let env = Environment::parse(&config_raw)
    //    .unwrap_or_else(|err| panic!("Could not parse config file with error: {:?}", err));

    //debug!("Environment: \n{}", env.env);

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
