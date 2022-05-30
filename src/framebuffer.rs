use log::debug;
use uefi::{
    prelude::BootServices,
    proto::console::gop::{GraphicsOutput, Mode, ModeInfo},
};

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
