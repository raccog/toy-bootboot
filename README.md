# Toy BOOTBOOT Implementation

This is a toy implementation of the BOOTBOOT protocol for x86_64 UEFI systems.

It is a work in progress and an experimental project.
My main goal is to see what advantages and disadvantages there are in using Rust to make freestanding programs; both in safety and in abstractions.

If you want a non-experimental boot loader implementing the BOOTBOOT protocol, use the [official reference implementation](https://gitlab.com/bztsrc/bootboot).

## Implementation Details

### Supported File Systems

Currently, [ustar](https://en.wikipedia.org/wiki/Tar_(computing)) (commonly known as tar) is the only supported file system for initrd.

### Boot Process

The boot loading process is as follows:

1. Read initrd file to memory
1. Search for config on ESP (EFI System Partition)
	1. If found, read config file to memory
	1. If not found, search for config on initrd file system
	2. If still not found, empty environment will be created
1. Create BOOTBOOT environment
	1. If config found, parse at linker-specified address
	1. If config not found, create empty environment
2. Search for kernel image
	1. If ramdisk is a file system, open kernel file using config path
	1. If ramdisk is not a filesystem, search for EFI header (fallback driver)
1. Initialize environment (ACPI, APIC, framebuffer, SMP, etc.)
2. Get memory map
1. Create BOOTBOOT header
1. Map kernel to dynamic address (specified in environment variable)
2. Map environment to static address (specified in source code)
3. Map BOOTBOOT header to static address (specified in source code)
1. Pass control to kernel entry point

## Initializing Environment

NOTE: This order may not be the order of execution when implemented.

1. Get framebuffer
2. Initialize SMP with all cores
3. Get timezone
4. Get timestamp of boot
6. Init APIC
9. Mask hardware interrupts
10. Enable FPU and SIMD
11. Enable virtual memory
12. Init BSS and stack
