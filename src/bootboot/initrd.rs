use alloc::vec::Vec;
use core::str::from_utf8;

/// BOOTBOOT initrd
#[repr(C)]
#[derive(Clone)]
pub struct Initrd {
    initrd: Vec<u8>,
}

impl Initrd {
    pub fn new(initrd: Vec<u8>) -> Self {
        Self { initrd }
    }

    fn read_ustar(&self, filename: &str) -> Option<&[u8]> {
        let mut initrd: &[u8] = &self.initrd[..];
        const NAME_OFFSET: usize = 157;
        const NAME_SIZE: usize = 100;
        const SIZE_OFFSET: usize = 124;
        const SIZE_SIZE: usize = 12;
        const FILE_OFFSET: usize = 512;

        while initrd.len() > 512 {
            let name = from_utf8(&initrd[NAME_OFFSET..NAME_OFFSET + NAME_SIZE])
                .unwrap_or_else(|_| panic!("Invalid ascii filename in initrd ustar"));
            let file_size = get_octal(from_utf8(&initrd[SIZE_OFFSET..SIZE_OFFSET + SIZE_SIZE])
                                 .unwrap_or_else(|_| panic!("Invalid size in initrd ustar")));
            if filename == name {
                return Some(&initrd[FILE_OFFSET..FILE_OFFSET + file_size]);
            }
            initrd = &initrd[(FILE_OFFSET + file_size) / 512 * 512..];
        }

        None
    }

    pub fn read_file(&self, filename: &str) -> Option<&[u8]> {
        self.read_ustar(filename)
    }
}

fn pow(x: usize, n: usize) -> usize {
    let mut result = x;
    for i in 2..=n {
        result *= x;
    }
    result
}

fn get_octal(octal: &str) -> usize {
    let mut size: usize = 0;

    for i in 0..12 {
        let digit = octal.chars().nth(i).unwrap().to_digit(8).unwrap() as usize;
        size += pow(digit, 12 - i);
    }

    size
}
