#!/bin/bash

# Create sysroot directory
SYSROOT="target/sysroot"
EFI="$SYSROOT/EFI/BOOT"
mkdir -p $EFI
# TODO: Allow release version to run
cp target/x86_64-unknown-uefi/debug/toy-bootboot.efi $EFI/BOOTX64.EFI

# Copy BOOTBOOT config
BOOTBOOT="$SYSROOT/BOOTBOOT"
CONFIG="$BOOTBOOT/CONFIG"
mkdir -p $BOOTBOOT
cp test/CONFIG $CONFIG

mkdir -p .cache

# Download OVMF UEFI firmware
CACHE=".cache"
OVMF="$CACHE/DEBUGX64_OVMF.fd"
if [[ ! -f $OVMF ]]; then
	wget https://retrage.github.io/edk2-nightly/bin/DEBUGX64_OVMF.fd
	mv DEBUGX64_OVMF.fd $OVMF
fi

# Download example kernel
CORE="$CACHE/sys/core"
mkdir -p $CACHE/sys
if [[ ! -f $CORE ]]; then
	wget 'https://gitlab.com/bztsrc/bootboot/-/raw/binaries/mykernel/mykernel.x86_64.elf' -O core
	mv core $CORE
fi

# Create initrd
INITRD="$BOOTBOOT/INITRD"
if [[ ! -f $INITRD ]]; then
	tar -C $CACHE --posix -cf INITRD sys/core
	mv INITRD $INITRD
fi

# Run bootloader
qemu-system-x86_64 \
	-bios .cache/DEBUGX64_OVMF.fd \
	-net none \
	-drive file=fat:rw:$SYSROOT,media=disk,format=raw \
	-m 128M \
	-serial stdio \
	-no-reboot -no-shutdown
