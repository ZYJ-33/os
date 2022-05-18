set arch riscv:rv64
target remote localhost:1234
symbol-file ./target/riscv64gc-unknown-none-elf/release/kernel
b kernel::rust_main
