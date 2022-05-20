#!/bin/bash

# Create sysroot
mkdir -p target/sysroot/EFI/BOOT
cp target/x86_64-unknown-uefi/debug/toy-bootboot.efi target/sysroot/EFI/BOOT/BOOTX64.EFI

# Create BOOTBOOT config
mkdir -p target/sysroot/BOOTBOOT
cp test/CONFIG target/sysroot/BOOTBOOT

# Download OVMF UEFI firmware
if [[ ! -f .cache/DEBUGX64_OVMF.fd ]]; then
	wget https://retrage.github.io/edk2-nightly/bin/DEBUGX64_OVMF.fd
	mv DEBUGX64_OVMF.fd .cache/
fi

# Run bootloader
qemu-system-x86_64 \
	-bios .cache/DEBUGX64_OVMF.fd \
	-net none \
	-drive file=fat:rw:target/sysroot,media=disk,format=raw \
	-m 128M \
	-serial stdio
	-no-reboot -no-shutdown
