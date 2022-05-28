use alloc::string::{String, ToString};
use core::str::FromStr;

// Since length does not include null terminator, max length is 4KiB - 1 or 4095 bytes
const ENVIRONMENT_MAX_LEN: usize = 4095;
const SCREEN_MIN_WIDTH: usize = 640;
const SCREEN_MIN_HEIGHT: usize = 480;

#[derive(Clone, Copy, Debug)]
pub enum ParseError {
    TooLarge,
}

/// Bootboot environment
pub struct Environment {
    pub env: String,
    pub screen: (usize, usize),
    pub kernel: String,
    pub no_smp: bool,
}

impl Default for Environment {
    fn default() -> Self {
        Environment {
            env: "kernel=sys/core\nscreen=1024x768".to_string(),
            screen: (1024, 768),
            kernel: "sys/core".to_string(),
            no_smp: false,
        }
    }
}

impl FromStr for Environment {
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Return error if environment is too large
        if s.len() > ENVIRONMENT_MAX_LEN {
            return Err(ParseError::TooLarge);
        }

        // Parse environment
        let env = String::from(s);
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
            if i >= env.len() {
                break;
            }

            // Get next char
            let c = env.chars().nth(i).unwrap();

            // Skip whitespace
            match c {
                ' ' | '\t' | '\r' | '\n' => continue,
                _ => {}
            }

            // Skip single-line comments
            if env[i..].starts_with("//") || env[i..].starts_with('#') {
                while i < env.len() {
                    i += 1;
                    if env[i..].starts_with('\n') {
                        break;
                    }
                }
                continue;
            }

            // Skip multi-line comments
            if env[i..].starts_with("/*") {
                while i < env.len() {
                    i += 1;
                    if env[i..].starts_with("*/") {
                        i += 1;
                        break;
                    }
                }
                continue;
            }

            // Ensure match is at start of line
            if i > 0 {
                match env.chars().nth(i - 1).unwrap() {
                    ' ' | '\t' | '\r' | '\n' => {}
                    _ => continue,
                }
            }

            // Get screen size
            let screen_key = "screen=";
            if env[i..].starts_with(screen_key) {
                // Get length of width in characters
                i += screen_key.len();
                let width_offset = env[i..].find('x');
                if width_offset.is_none() {
                    continue;
                }
                let width_offset = width_offset.unwrap();

                // Parse screen width
                let width = env[i..i + width_offset].parse::<usize>();
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
                let height_offset = env[i..].find(char::is_whitespace);
                if height_offset.is_none() {
                    continue;
                }
                let height_offset = height_offset.unwrap();

                // Parse height
                let height = env[i..i + height_offset].parse::<usize>();
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
                while i < env.len() {
                    if env[i..].starts_with('\n') {
                        break;
                    }
                    i += 1;
                }
                continue;
            }

            // Get kernel filename
            let kernel_key = "kernel=";
            if env[i..].starts_with(kernel_key) {
                i += kernel_key.len();
                // Ensure not at end of file
                if i >= env.len() {
                    continue;
                }
                // Skip whitespace until kernel path starts
                let mut j = i;
                while j < env.len() {
                    if env[j..].starts_with(char::is_whitespace) {
                        break;
                    }
                    j += 1;
                }
                // Set kernel filename
                if j - i >= 1 {
                    kernel_filename = &env[i..j];
                }
                i = j;
                continue;
            }

            // Check for smp disable
            let smp_disable_key = "nosmp=1";
            if env[i..].starts_with(smp_disable_key) {
                i += smp_disable_key.len();
                no_smp = true;
            }
        }

        let mut kernel = String::with_capacity(kernel_filename.as_bytes().len());
        kernel.push_str(kernel_filename);
        Ok(Environment {
            env,
            screen,
            kernel,
            no_smp,
        })
    }
}
