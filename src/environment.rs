use alloc::string::{String, ToString};
use log::debug;
use uefi::{
    prelude::Status,
    proto::media::file::{Directory, FileAttribute, FileMode},
    Error as UefiError, Result as UefiResult,
};

use crate::{open_file, read_to_string, Initrd};

// Since length does not include null terminator, max length is 4KiB - 1 or 4095 bytes
const ENVIRONMENT_MAX_SIZE: usize = 4095;
const SCREEN_MIN_WIDTH: usize = 640;
const SCREEN_MIN_HEIGHT: usize = 480;

/// An error that occurred while parsing a config file.
#[derive(Clone, Copy, Debug)]
pub enum ParseError {
    TooLarge,
}

/// Bootboot environment.
///
/// Contains:
///
/// * Preferred screen resolution
/// * Kernel file name in initrd
/// * Flag showing whether SMP is disabled
pub struct Environment {
    pub env_raw: String,
    pub screen: (usize, usize),
    pub kernel: String,
    pub no_smp: bool,
}

impl Environment {
    /// Returns the BOOTBOOT environment to pass to the kernel.
    ///
    /// The following steps are run until a valid environment is returned:
    ///
    /// 1. Try to read `BOOTBOOT/CONFIG` from boot partition and parse environment.
    /// 2. Try to read `sys/config` from `initrd` and parse environment.
    /// 3. If neither file contains a valid environment, return a default environment.
    pub fn get_env(bootdir: &mut Directory, initrd: &Initrd) -> Self {
        // Try to parse environment, first from boot disk, then from initrd
        if let Ok(env_raw) = get_env_raw(bootdir, initrd) {
            if let Ok(env) = Self::from_string(env_raw) {
                return env;
            }
        }

        // Return default environment if environment could not be read
        debug!("Using default environment");
        Self::default()
    }

    /// Parses a raw config file to obtain a BOOTBOOT environment.
    ///
    /// # Errors
    ///
    /// Returns an error if the raw config file is larger than 4KiB.
    pub fn from_string(env_raw: String) -> Result<Self, ParseError> {
        // Return error if environment is too large
        if env_raw.as_bytes().len() > ENVIRONMENT_MAX_SIZE {
            return Err(ParseError::TooLarge);
        }

        // Parse environment
        let mut i: usize = 0;
        let mut screen: (usize, usize) = (1024, 768); // default screen size
        let mut kernel_filename = "sys/core";
        let mut no_smp = false;
        loop {
            // Increment unless at start
            // This is done at the beginning of the loop so that it does not need to be put before
            // every continue statement
            if i > 0 {
                i += 1;
            }

            // Break at end of file
            if i >= env_raw.len() {
                break;
            }

            // Get next char
            let c = env_raw.chars().nth(i).unwrap();

            // Skip whitespace
            match c {
                ' ' | '\t' | '\r' | '\n' => continue,
                _ => {}
            }

            // Skip single-line comments
            if env_raw[i..].starts_with("//") || env_raw[i..].starts_with('#') {
                while i < env_raw.len() {
                    i += 1;
                    if env_raw[i..].starts_with('\n') {
                        break;
                    }
                }
                continue;
            }

            // Skip multi-line comments
            if env_raw[i..].starts_with("/*") {
                while i < env_raw.len() {
                    i += 1;
                    if env_raw[i..].starts_with("*/") {
                        i += 1;
                        break;
                    }
                }
                continue;
            }

            // Ensure match is at start of line
            if i > 0 {
                match env_raw.chars().nth(i - 1).unwrap() {
                    ' ' | '\t' | '\r' | '\n' => {}
                    _ => continue,
                }
            }

            // Get screen size
            let screen_key = "screen=";
            if env_raw[i..].starts_with(screen_key) {
                // Get length of width in characters
                i += screen_key.len();
                let width_offset = env_raw[i..].find('x');
                if width_offset.is_none() {
                    continue;
                }
                let width_offset = width_offset.unwrap();

                // Parse screen width
                let width = env_raw[i..i + width_offset].parse::<usize>();
                if width.is_err() {
                    continue;
                }
                let width = width.unwrap();

                // Ensure screen width is valid
                let width = if width < SCREEN_MIN_WIDTH {
                    SCREEN_MIN_WIDTH
                } else {
                    width
                };

                // Get offset to height
                i += width_offset + 1;
                let height_offset = env_raw[i..].find(char::is_whitespace);
                if height_offset.is_none() {
                    continue;
                }
                let height_offset = height_offset.unwrap();

                // Parse height
                let height = env_raw[i..i + height_offset].parse::<usize>();
                if height.is_err() {
                    continue;
                }
                let height = height.unwrap();
                i += height_offset;

                // Ensure screen height is valid
                let height = if height < SCREEN_MIN_HEIGHT {
                    SCREEN_MIN_HEIGHT
                } else {
                    height
                };

                // Set screen resolution
                screen = (width, height);

                // Skip characters until new line is found
                while i < env_raw.len() {
                    if env_raw[i..].starts_with('\n') {
                        break;
                    }
                    i += 1;
                }
                continue;
            }

            // Get kernel filename
            let kernel_key = "kernel=";
            if env_raw[i..].starts_with(kernel_key) {
                i += kernel_key.len();
                // Ensure not at end of file
                if i >= env_raw.len() {
                    continue;
                }
                // Skip whitespace until kernel path starts
                let mut j = i;
                while j < env_raw.len() {
                    if env_raw[j..].starts_with(char::is_whitespace) {
                        break;
                    }
                    j += 1;
                }
                // Set kernel filename
                if j - i >= 1 {
                    kernel_filename = &env_raw[i..j];
                }
                i = j;
                continue;
            }

            // Check for smp disable
            let smp_disable_key = "nosmp=1";
            if env_raw[i..].starts_with(smp_disable_key) {
                i += smp_disable_key.len();
                no_smp = true;
            }
        }

        let kernel = String::from(kernel_filename);
        Ok(Environment {
            env_raw,
            screen,
            kernel,
            no_smp,
        })
    }
}

impl Default for Environment {
    fn default() -> Self {
        Environment {
            env_raw: "kernel=sys/core\nscreen=1024x768".to_string(),
            screen: (1024, 768),
            kernel: "sys/core".to_string(),
            no_smp: false,
        }
    }
}

/// Returns the contents of a config file.
///
/// # Errors
///
/// Returns an error if initrd could not be read from the boot disk or initrd.
fn get_env_raw(bootdir: &mut Directory, initrd: &Initrd) -> UefiResult<String> {
    read_env_file(bootdir).or(read_env_initrd(initrd))
}

/// Returns the contents of `BOOTBOOT/CONFIG` if the file exists on the boot disk.
///
/// # Errors
///
/// Returns an error if the file could not be read.
fn read_env_file(bootdir: &mut Directory) -> UefiResult<String> {
    let mut env_file = open_file(bootdir, "CONFIG", FileMode::Read, FileAttribute::empty())?;
    let env = read_to_string(&mut env_file);

    if env.is_ok() {
        debug!("Found environment on boot disk");
    }
    env
}

/// Returns the contents of `sys/config` if the file exists on initrd.
///
/// # Errors
///
/// Returns an error if the file could not be read.
fn read_env_initrd(initrd: &Initrd) -> UefiResult<String> {
    let env_file = initrd.read_file("sys/config").ok_or(Status::NOT_FOUND)?;
    let env =
        String::from_utf8(env_file.to_vec()).map_err(|_| UefiError::from(Status::VOLUME_CORRUPTED));

    if env.is_ok() {
        debug!("Found environment in initrd file 'sys/config'");
    }
    env
}
