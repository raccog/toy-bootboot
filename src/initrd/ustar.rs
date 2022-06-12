use core::str;

const BLOCK_SIZE: usize = 512;
const NAME_SIZE: usize = 100;
const SIZE_OFFSET: usize = 124;
const SIZE_SIZE: usize = 12;

/// Tries to read `filename` from initrd; a tar archive.
///
/// Returns `None` if initrd is not a valid tar archive or if `filename` is not a valid file in the
/// archive.
pub fn read_ustar<'a>(initrd: &'a [u8], filename: &str) -> Option<&'a [u8]> {
    let mut idx = 0;

    while idx < initrd.len() && idx + BLOCK_SIZE <= initrd.len() {
        // Get block header
        let header = &initrd[idx..idx + BLOCK_SIZE];
        idx += BLOCK_SIZE;
        // Get file size octal string
        let file_size = read_octal_size(
            header[SIZE_OFFSET..SIZE_OFFSET + SIZE_SIZE - 1]
                .try_into()
                .unwrap(),
        );
        if file_size.is_none() {
            continue;
        }
        let file_size = file_size.unwrap();

        // Get filename
        let name = str::from_utf8(&header[..NAME_SIZE]);
        if name.is_err() {
            continue;
        }
        // Trim trailing null characters
        let name = name.unwrap().trim_end_matches('\0');

        // Return file contents if names match and file has valid size
        if filename == name && idx + file_size <= initrd.len() {
            return Some(&initrd[idx..idx + file_size]);
        }

        // Move index past file data
        let extra_block_size = file_size % BLOCK_SIZE;
        let block_padding = if extra_block_size > 0 {
            BLOCK_SIZE - extra_block_size
        } else {
            0
        };
        idx += file_size + block_padding;
    }

    None
}

fn pow(x: usize, n: usize) -> usize {
    if n == 0 {
        return 1;
    }

    let mut result = x;
    for _ in 2..=n {
        result *= x;
    }
    result
}

fn read_octal_size(octal_str: [u8; SIZE_SIZE - 1]) -> Option<usize> {
    let mut size = 0;
    let octal_str = str::from_utf8(&octal_str[..]).ok()?;

    for (i, c) in octal_str.chars().enumerate() {
        let digit = c.to_digit(8)? as usize;
        size += digit * pow(8, 10 - i);
    }

    Some(size)
}
