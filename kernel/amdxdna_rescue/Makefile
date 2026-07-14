# Out-of-tree rescue module. Build against the *running* kernel.
#
#   make
#   sudo insmod amdxdna_rescue.ko action=diagnose
#   sudo dmesg -T | tail -80
#   sudo rmmod amdxdna_rescue

KDIR ?= /lib/modules/$(shell uname -r)/build
PWD  := $(shell pwd)
# Prefer binutils ld.bfd — mold/wild can fail on kernel module.lds
LD_BFD := $(shell command -v ld.bfd 2>/dev/null || echo ld)

obj-m += amdxdna_rescue.o

all:
	$(MAKE) -C $(KDIR) M=$(PWD) LD=$(LD_BFD) modules

clean:
	$(MAKE) -C $(KDIR) M=$(PWD) LD=$(LD_BFD) clean

.PHONY: all clean
