#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
use core::arch::asm;

#[no_mangle]
fn main() -> i32
{
    println!("use privilage instr");
    unsafe
    {
        asm!("sret");
    }
    0
}