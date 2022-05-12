/// BOOTBOOT initrd
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Initrd {
    pub ptr: u64,
    pub size: u64,
}
