<p align="center">
<img src="media/FFFuzzer.jpg">
</p>

# FFFuzzer

**FFFuzzer** is a fork of the FuzzNG project, with the goal of enhancing the original project's functionality.

The new features include:

- [ ] Web data dashboard
- [ ] Comments starting with `//#`
- [ ] Code optimization

# Instructions
These instructions were tested on Ububtu22. A CPU with VT-x support is preferable.
The user needs rw permissions for /dev/kvm

Install Requirements:
```bash
# QEMU:
sudo apt-get install git libglib2.0-dev libfdt-dev libpixman-1-dev zlib1g-dev ninja-build

# Kernel:
sudo apt-get install build-essential linux-source bc kmod cpio flex libncurses5-dev libelf-dev libssl-dev dwarves bison libcap-ng-dev libattr1-dev

# Misc:
sudo apt install clang-15 debootstrap qemu-utils
```

Build Kernel + FuzzNG (mod-ng/qemu-ng/libfuzzer-ng/agent-ng)

Note that clang is required.
```bash
NPROC=4 CC=clang-15 CXX=clang++-15 make
# This may ask for your password to set up the disk-image for the fuzzing VM.
```

Now pick a fuzzing-config from `configs/` and start the fuzzer:

```bash
# Fuzz KVM with 4 workers
./scripts/fuzz.sh 4 configs/kvm.h
```

Or, to run a single worker with serial-output from the VM enabled:
```bash
# Manually copy the KVM config:
cp configs/kvm.h agent/fuzz_config.h

# Run a fuzzer
EXTRA_ARGS="-serial stdio" PROJECT_ROOT="./" ./scripts/run.sh
```