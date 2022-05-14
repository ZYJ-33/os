TARGET := riscv64gc-unknown-none-elf
PROJ_NAME := kernel
MODE := release
ELF := ./target/$(TARGET)/$(MODE)/$(PROJ_NAME)
BIN := ./target/$(TARGET)/$(MODE)/$(PROJ_NAME).bin
DIS_FILE := ./$(PROJ_NAME).asm
OTHER_DIS_FILE := ./other_$(PROJ_NAME).asm
BIOS := ../bootloader/rustsbi-qemu.bin

OBJ_DUMP := rust-objdump --arch-name=riscv64
OBJ_COPY := rust-objcopy --binary-architecture=riscv64
DIS_FLAG := -d
KERNEL_START_ADDR = 0x80200000

QEMU_PARA := -machine virt -nographic -bios $(BIOS) -device loader,file=$(BIN),addr=$(KERNEL_START_ADDR)

user:
	cd ../user && make build

kernel: user
	cargo build --$(MODE)

$(BIN): kernel
	@$(OBJ_COPY) $(ELF) --strip-all -O binary $@

$(DIS_FILE): kernel
	@$(OBJ_DUMP) $(DIS_FLAG) $(ELF) > $(DIS_FILE)

other_dis: kernel
	@$(OBJ_DUMP) -t $(ELF) > $(OTHER_DIS_FILE)

build: $(BIN) $(DIS_FILE) other_dis

clean: 
	@cargo clean
	rm $(DIS_FILE)
	rm $(OTHER_DIS_FILE)
	rm $(BIN)

run: build
	qemu-system-riscv64 $(QEMU_PARA)

gdb: build
	qemu-system-riscv64 $(QEMU_PARA) -s -S

.PHONY: kernel user $(BIN) $(DIS_FILE) other_dis clean build
