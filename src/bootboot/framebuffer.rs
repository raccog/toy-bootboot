/// BOOTBOOT linear framebuffer
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Framebuffer {
    pub ptr: u64,
    pub size: u32,
    pub width: u32,
    pub height: u32,
    pub scanline: u32,
}
