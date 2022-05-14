    .align 3
    .section .data
    .global apps

apps:
    .quad app0_start
    .quad app1_start
    .quad app2_start
    .quad app3_start
    .quad app4_start
    .quad app4_end

.section .data
.global app0_start
app0_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/helloworld.bin"
app0_end:

.section .data
.global app1_start
app1_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/store_fault.bin"
app1_end:

.section .data
.global app2_start
app2_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/power.bin"
app2_end:

.section .data
.global app3_start
app3_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/priv_instr.bin"
app3_end:

.section .data
.global app4_start
app4_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/priv_reg.bin"
app4_end:
