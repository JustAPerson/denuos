# Denuos is rather straight forward to build.
# Just run the following commands
#
#   make iso
#   make run
#
# This will compile all the source code, burn an ISO image, and run it under
# QEMU. Other useful targets include:
#
#   debug - run QEMU paused, waiting for a GDB connection
#   clean - remove all build objects
#
# The final product ISO file will be written to ./bin/denuos.iso

.PHONY: run debug clean iso

isofile := ./bin/denuos.iso

QEMU ?= qemu-system-x86_64
run: $(isofile)
	$(QEMU) $(QEMUFLAGS) -cdrom $<

# See ./doc/Debugging.md for more info
debug: QEMUFLAGS += -s -S -d int -no-reboot
debug: run

clean:
	rm -rf ./bin/

iso: $(isofile)

kernelbin := ./bin/kernel.bin
isofs   := ./bin/iso
grubcfg := ./src/arch/x86/grub.cfg
$(isofile): $(kernelbin)
	mkdir -p $(isofs)
	mkdir -p $(isofs)/boot/grub
	cp $(kernelbin) $(isofs)/boot/kernel.bin
	cp $(grubcfg) $(isofs)/boot/grub/grub.cfg
	grub-mkrescue -o $(isofile) $(isofs) 2>/dev/null

target := x86_64-unknown-linux-gnu
kernel_cargo := ./bin/cargo/kernel
kernelobj := $(kernel_cargo)/$(target)/debug/libkernel.a
$(kernelobj): export CARGO_TARGET_DIR := $(abspath $(kernel_cargo))
$(kernelobj): .FORCE | ./bin/cargo
	cd src/kernel/ && cargo build --target $(target)

bootsrcs := multiboot.s boot32.s boot64.s
bootobjs := $(bootsrcs:%.s=%.o)
bootobjs := $(addprefix ./bin/boot/, $(bootobjs))
$(kernelbin): $(bootobjs) $(kernelobj)
	ld --gc-sections -n -T ./src/arch/x86/link.ld -o $@ $^

./bin/boot/%.o: ./src/arch/x86/%.s | ./bin/boot
	nasm -f elf64 -o $@ $<


./bin/boot ./bin/cargo:
	mkdir -p $@

.FORCE:
