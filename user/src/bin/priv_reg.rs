#![no_std]
#![no_main]

use riscv::register::sstatus::{self, SPP};

#[macro_use]
extern crate user_lib;

#[no_mangle]
fn main() -> i32
{
    println!("try to write a priv reg");
    unsafe
    {
        sstatus::set_spp(SPP::Supervisor);
    }
    1
}


