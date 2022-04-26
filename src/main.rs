#![no_std]
#![no_main]
#![feature(abi_efiapi)]

mod bootboot;
mod environment;

use bootboot::BootbootHeader;
use environment::get_environment;

use log::{debug,error};
use uefi::{
    CStr16,
    prelude::*,
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

    let mut env: [u8; 4096] = [0; 4096];
    let size = get_environment(image_handle, &mut st, &mut env); 
    if let Err(size) = size {
        error!("Config file of size {} could not fit in a page", size);
        return Status::BAD_BUFFER_SIZE;
    }
    let size = size.unwrap();

    let mut buf: [u16; 4096] = [0; 4096];
    for i in 0..4096 {
        buf[i] = env[i] as u16;
    }

    let out = CStr16::from_u16_with_nul(&buf[0..size]).expect("Could not convert environment to CStr16");
    debug!("Environment:\n{}", out);

    loop {}

    Status::SUCCESS
}
