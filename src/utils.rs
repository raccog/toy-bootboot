use core::{mem, num::Wrapping, slice};

/// An error resulting from parsing ACPI or SMBIOS tables.
pub enum ParseError {
    FailedChecksum,
    InvalidPointer,
    InvalidSignature,
    InvalidSize,
    NoTable,
    TooLarge,
}

/// A type that implements a checksum.
pub trait Checksum {
    /// Gets the sum of every byte that composes this struct and returns the least significant
    /// byte.
    fn checksum(&self) -> u8
    where
        Self: Sized,
    {
        let data = unsafe {
            slice::from_raw_parts((self as *const Self) as *const u8, mem::size_of::<Self>())
        };
        checksum(data)
    }

    /// Returns true if the checksum equals 0.
    fn checksum_valid(&self) -> bool
    where
        Self: Sized,
    {
        self.checksum() == 0
    }
}

/// Gets the sum of every byte in `data` and returns the least significant byte.
pub fn checksum(data: &[u8]) -> u8 {
    let mut sum: Wrapping<u8> = Wrapping(0);
    for b in data {
        sum += b;
    }

    sum.0
}

/// Trait for types with magic numbers.
///
/// Magic numbers are also called signatures or anchors.
pub trait Magic<const SIZE: usize> {
    /// Returns this instance's magic numbers.
    fn magic(&self) -> &[u8; SIZE];
}
