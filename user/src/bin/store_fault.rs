#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

#[no_mangle]
fn main() -> i32
{
    println!("store fault app");
    unsafe
    {
        core::ptr::null_mut::<u8>().write_volatile(0);
    }
    0
}

