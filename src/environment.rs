use alloc::string::String;
use core::{
    iter::Peekable,
    str::{Chars, FromStr}
};

// Since length does not include null terminator, max length is 4KiB - 1 or 4095 bytes
const ENVIRONMENT_MAX_LEN: usize = 4095;

#[derive(Clone, Copy, Debug)]
pub enum ParseError {
    InvalidScreen,
    TooLarge,
}

/// Bootboot environment
pub struct Environment {
    pub env: String,
    pub screen: (usize, usize),
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
        let mut screen_size: (usize, usize) = (1024, 768);  // default screen size
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
            if env[i..].starts_with("//") || env[i..].starts_with("#") {
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
                    ' ' | '\t' | '\r' | '\n' => {},
                    _ => continue
                }
            }

            // Get screen size
            let screen_key = "screen=";
            if env[i..].starts_with(screen_key) {
                i += screen_key.len();
                let width_offset = env[i..].find('x');
                if width_offset.is_none() {
                    return Err(ParseError::InvalidScreen);
                }
                let width_offset = width_offset.unwrap();

                let width = env[i..i + width_offset].parse::<usize>();
                if width.is_err() {
                    return Err(ParseError::InvalidScreen);
                }
                let width = width.unwrap();

                i += width_offset + 1;
                let height_offset = env[i..].find(char::is_whitespace);
                if height_offset.is_none() {
                    return Err(ParseError::InvalidScreen);
                }
                let height_offset = height_offset.unwrap();

                let height = env[i..i + height_offset].parse::<usize>();
                if height.is_err() {
                    return Err(ParseError::InvalidScreen);
                }
                let height = height.unwrap();
                i += height_offset;

                screen_size = (width, height);

                while i < env.len() {
                    if env[i..].starts_with('\n') {
                        break;
                    }
                    i += 1;
                }
                continue;
            }
        }

        Ok(Environment { env, screen: screen_size })
    }
}
