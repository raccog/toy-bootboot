use alloc::vec::Vec;
use core::str::from_utf8;
use log::debug;

const BLOCK_SIZE: usize = 512;

pub fn read_ustar<'a>(initrd: &'a Vec<u8>, filename: &str) -> Option<&'a [u8]> {
    let mut initrd: &[u8] = &initrd[..];
    const NAME_SIZE: usize = 100;
    const SIZE_OFFSET: usize = 124;
    const SIZE_SIZE: usize = 12;
    const FILE_OFFSET: usize = BLOCK_SIZE;

    while initrd.len() > BLOCK_SIZE {
        // Get file size octal string
        let file_size = from_utf8(&initrd[SIZE_OFFSET..SIZE_OFFSET + SIZE_SIZE]);
        if file_size.is_err() {
            initrd = &initrd[BLOCK_SIZE..];
            continue;
        }
        // Parse octal string for file size
        let file_size = get_octal(file_size.unwrap());
        if file_size.is_none() {
            initrd = &initrd[BLOCK_SIZE..];
            continue;
        }
        let file_size = file_size.unwrap();

        // Get filename
        let name = from_utf8(&initrd[..NAME_SIZE]);
        if name.is_err() {
            initrd = &initrd[BLOCK_SIZE..];
            continue;
        }
        let name = name.unwrap().trim_end_matches('\0');

        // Return file contents if names match
        if filename == name {
            return Some(&initrd[FILE_OFFSET..FILE_OFFSET + file_size]);
        }

        // Check next file metadata block
        let extra_block_size = file_size % BLOCK_SIZE;
        let block_padding = if extra_block_size > 0 {
            BLOCK_SIZE - extra_block_size
        } else {
            0
        };
        initrd = &initrd[file_size + block_padding..];
    }

    None
}

fn pow(x: usize, n: usize) -> usize {
    if n == 0 {
        return 1;
    }

    let mut result = x;
    for i in 2..=n {
        result *= x;
    }
    result
}

fn get_octal(octal: &str) -> Option<usize> {
    let mut size: usize = 0;

    for i in 0..11 {
        let digit = octal.chars().nth(i)?.to_digit(8)? as usize;
        size += digit * pow(8, 10 - i);
    }

    Some(size)
}
