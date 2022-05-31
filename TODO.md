# Todo List for Project

## Immediate

### Multiprocessor Initialization

* Get multiprocessor services from ACPI table
* Start kernel on core 0
* Start trampoline on each extra core

### Parse Kernel ELF File

* Get kernel from initrd
* Parse kernel ELF header
* Copy kernel into mapped memory

### Memory Mapping

* Setup initial page tables

## Initializing Hardware

NOTE: This order may not be the order of execution when implemented.

- [x] Framebuffer
- [ ] Multiprocessors
- [x] Timestamp and timezone
- [x] Get ACPI table
- [x] Get SMBIOS table
- [ ] Map memory
- [ ] Init BSS and stack

## Future

### Memory Allocation

* Create custom allocator

### Running and Testing Bootloader

* Research into different ways to run and test bootloader.

### Misc

* More documentation
* Create custom logger
