# Toy BOOTBOOT Implementation

This is a toy implementation of the BOOTBOOT protocol.

For the official reference implementation, go [here](https://gitlab.com/bztsrc/bootboot).

## Implementation Details

NOTE: Choose a filesystem driver for ramdisk

1. Find config on ESP (EFI System Partition)
2. Read and parse config settings
3. Pass config environment to memory at linker-specified address
1. Find ramdisk on ESP
1. Read ramdisk data to memory (somewhere in first 16G)
1. If ramdisk is a filesystem (a single filesystem driver will be implemented for now), open kernel file using config path
2. If ramdisk is not a filesystem, search for EFI header (fallback driver)
3. Map kernel to static memory address (specified in config settings)
4. Initialize environment
5. Pass control to kernel entry point

## Initializing Environment

NOTE: This order may not be the order of execution.

1. Get framebuffer
2. Initialize SMP with all cores
3. Get timezone
4. Get timestamp of boot
6. Init APIC
7. Init serial output
7. Map BOOTBOOT header to linker-specified address
7. Get memory map
8. Map memory map to linker-specified address
9. Mask hardware interrupts
10. Enable FPU and SIMD
11. Enable virtual memory
12. Init BSS and stack