# Toy BOOTBOOT Implementation

This is a toy implementation of the BOOTBOOT protocol for x86_64 UEFI systems.

It is a work in progress and an experimental project. My main goal is to see what advantages and disadvantages there are in using Rust to make freestanding programs; in safety, abstractions, and tooling.

If you want a non-experimental boot loader implementing the BOOTBOOT protocol, use the [official reference implementation](https://gitlab.com/bztsrc/bootboot).

## Implementation Details

### Supported File Systems

Currently, [ustar](https://en.wikipedia.org/wiki/Tar_(computing)) (commonly known as tar) is the only supported file system for initrd.

### Boot Process

The boot loading process is as follows:

1. Read initrd file to memory
2. Get BOOTBOOT environment
	1. Try to parse from `BOOTBOOT/CONFIG` on boot partition
	2. If file not found, try to parse from `sys/config` on initrd
	3. If still not found, default environment will be used
2. Search for kernel image
	1. If initrd is a file system, open kernel file using path from environment variable
	2. If initrd is not a filesystem, search for EFI header (fallback driver)
3. Initialize hardware (ACPI, APIC, framebuffer, SMP, etc.)
4. Get memory map
5. Create BOOTBOOT header
6. Map kernel to dynamic address (specified in environment variable)
7. Map environment to static address (specified in source code)
8. Map BOOTBOOT header to static address (specified in source code)
9. Pass control to kernel entry point
