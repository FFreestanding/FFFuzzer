
KVERSION=linux-6.1

NPROC ?=2

MKFILE_PATH := $(abspath $(lastword $(MAKEFILE_LIST)))
PROJECT_ROOT := $(dir $(MKFILE_PATH))

all: image kernelbuild qemubuild libfuzzerng agentbuild

# .:: kernel + mod-ng ::.
kernelbuild: kernel kernel/.config mod-ng
	cd $(PROJECT_ROOT)/kernel; \
		make -j$(NPROC) CC=$(CC) \
		KCFLAGS="-fsanitize-coverage-allowlist=$(PROJECT_ROOT)/kernel/whitelist"

mod-ng: kernel/include/linux/fuzzer_dev.h

kernel/include/linux/fuzzer_dev.h:
ifeq (,$(wildcard kernel/include/linux/fuzzer_dev.h))
	git apply -v --directory=kernel/ kernel-patches/*.patch
endif

kernel/.config:
	cp kernel-configs/general kernel/.config

kernel:
	# curl -s -L https://cdn.kernel.org/pub/linux/kernel/v6.x/$(KVERSION).tar.gz | tar -xz
	curl -s -L https://mirrors.aliyun.com/linux-kernel/v6.x/linux-6.1.tar.gz | tar -xz
	mv $(KVERSION) kernel

# .:: qemu-ng ::.
qemubuild: qemu/build/qemu-fuzz-x86_64

qemu/build/qemu-fuzz-x86_64: qemu/hw/i386/fuzz.c libfuzzerng qemu-build
	cd qemu-build; ninja -j$(NPROC) qemu-fuzz-x86_64

qemu-build:
	mkdir -p qemu-build; cd qemu-build; \
	LIB_FUZZING_ENGINE="$(PROJECT_ROOT)/libfuzzer-ng/libFuzzer.a" \
	$(PROJECT_ROOT)/qemu/configure --enable-fuzzing --enable-virtfs;
	#--enable-sanitizers

qemu/hw/i386/fuzz.c: qemu/
ifeq (,$(wildcard qemu/hw/i386/fuzz.c))
	cd $(PROJECT_ROOT)/qemu; git am $(PROJECT_ROOT)/qemu-patches/*.patch
endif

qemu/:
	git clone https://gitlab.com/qemu-project/qemu
	cd qemu; git checkout v8.0.0;

# .:: agent ::.
agentbuild:
	cd $(PROJECT_ROOT)/agent; make

# .:: libfuzzer-ng ::.
libfuzzerng: libfuzzer-ng/libFuzzer.a

libfuzzer-ng/libFuzzer.a:
	cd libfuzzer-ng; ./build.sh

# .:: image ::.
image: images/bullseye.img

images/bullseye.img:
	mkdir -p images
	cd images; $(PROJECT_ROOT)/scripts/create-image.sh
