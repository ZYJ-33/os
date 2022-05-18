    .align 3
    .section .data
    .global apps

apps:
    .quad 4
    .quad app0_start
    .quad app1_start
    .quad app2_start
    .quad app3_start
    .quad app3_end

.section .data
.global app0_start
app0_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/00power_3.bin"
app0_end:

.section .data
.global app1_start
app1_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/01power_5.bin"
app1_end:

.section .data
.global app2_start
app2_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/02power_7.bin"
app2_end:

.section .data
.global app3_start
app3_start:
    .incbin "../user/target/riscv64gc-unknown-none-elf/release/03sleep.bin"
app3_end:

