/// BOOTBOOT linear framebuffer
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Framebuffer {
    pub ptr: u64,
    pub size: u32,
    pub width: u32,
    pub height: u32,
    pub scanline: u32,
}

impl Framebuffer {
    pub fn new(ptr: u64, size: u32, width: u32, height: u32, scanline: u32) -> Self {
        Self {ptr, size, width, height, scanline}
    }
}
