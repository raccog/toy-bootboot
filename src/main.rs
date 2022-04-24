#![no_std]
#![no_main]
#![feature(abi_efiapi)]

use core::str;
use log::debug;
use uefi::{
    CString16,
    prelude::*,
    proto::media::file::{FileMode,FileAttribute,File,RegularFile}
};

#[cfg(debug_assertions)]
fn debug_firmware_info(st: &SystemTable<Boot>) {
    let fw_revision = st.firmware_revision();
    let uefi_revision = st.uefi_revision();

    debug!("UEFI firmware information:");
    debug!("Vendor = {}", st.firmware_vendor());
    debug!("Firmware Revision = {}.{}", fw_revision.major(), fw_revision.minor());
    debug!("UEFI Revision = {}.{}", uefi_revision.major(), uefi_revision.minor());
}

#[entry]
fn main(image_handle: Handle, mut st: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut st).unwrap();

    // Log debug statements if built in debug mode
    if cfg!(debug_assertions) {
        log::set_max_level(log::LevelFilter::Debug);
    }

    // Print firmware info if in debug mode
    debug_firmware_info(&st);

    // Boot table
    let bt = st.boot_services();
    // ESP file system
    let fs = unsafe {
        &mut *bt.get_image_file_system(image_handle)
        .expect("Failed to get ESP")
        .interface.get()
    };
    // Root directory on ESP
    let mut root = fs.open_volume()
        .expect("Failed to open root directory on ESP");
    
    // Test reading a file
    let filename = CString16::try_from("test.txt").unwrap();
    let file = root.open(&filename, FileMode::Read, FileAttribute::empty())
        .expect("Could not open file 'test.txt'");
    let mut text = unsafe { RegularFile::new(file) };

    let mut buf: [u8; 256] = [0; 256];
    let size = text.read(&mut buf)
        .expect("Could not read file");
    text.close();
    debug!("Size: {}", size);
    let out = unsafe { str::from_utf8_unchecked(&buf) };

    debug!("{}", out);
    //debug!("File size: {}", size);

    loop {}

    Status::SUCCESS
}
