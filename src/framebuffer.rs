use log::debug;
use uefi::{
    prelude::BootServices,
    proto::console::gop::{GraphicsOutput, ModeInfo},
    Result as UefiResult,
};

/// Uses UEFI Graphics Output Protocol to find an available graphics mode that closely matches the
/// `target_resolution`.
///
/// Returns the native mode if it matches the `target_resolution`.
///
/// If the mode that is closest to the `target_resolution` is not the native mode, then the GOP is
/// set to use the new mode. However, if this action fails then the native mode is returned.
///
/// # Errors
///
/// Returns an error if GOP cannot be located.
fn get_gop_info(bt: &BootServices, _target_resolution: (usize, usize)) -> UefiResult<ModeInfo> {
    // Try to get GOP (graphics output protocol)
    let gop = unsafe { &mut *bt.locate_protocol::<GraphicsOutput>()?.get() };

    // Get GOP native mode
    let native_info = gop.current_mode_info();
    debug!(
        "Native mode: resolution={:?}, stride={}, format={:?}",
        native_info.resolution(),
        native_info.stride(),
        native_info.pixel_format()
    );

    // Always use native video mode for now
    Ok(native_info)

    // Return native mode if it matches the target resolution
    // TODO: Decide on how to choose video mode when native does not match
}

/// BOOTBOOT linear framebuffer information.
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
    /// Uses UEFI Graphics Output Protocol to create a [`Framebuffer`] that most closely matches
    /// `target_resolution`.
    ///
    /// For now, the native resolution is always used.
    ///
    /// # Errors
    ///
    /// Returns an error if GOP cannot be located.
    pub fn from_boot_services(
        bt: &BootServices,
        target_resolution: (usize, usize),
    ) -> UefiResult<Self> {
        // Get GOP mode
        let gop_info = get_gop_info(bt, target_resolution)?;

        // Get GOP (graphics output protocol)
        let gop = unsafe { &mut *bt.locate_protocol::<GraphicsOutput>()?.get() };
        let mut uefi_framebuffer = gop.frame_buffer();
        let ptr = uefi_framebuffer.as_mut_ptr() as usize as u64;
        let (width, height) = gop_info.resolution();
        let size = uefi_framebuffer.size() as u32;

        // Create Framebuffer from GOP info
        Ok(Self {
            ptr,
            size,
            width: width as u32,
            height: height as u32,
            scanline: gop_info.stride() as u32,
        })
    }
}
