#![no_std]
#![no_main]
#![feature(global_asm)]
#![feature(asm)]
#![feature(panic_info_message)]


mod lang_items;
mod sbi;
#[macro_use]
mod console;
mod sync;
mod batch;
mod trap;
mod syscall;
use crate::{sbi::shutdown, console::print};
use trap::init as trap_init;
pub use batch::{init as proc_init, run_app};
pub use syscall::sys_call;
use::core::arch::global_asm;

global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("app.S"));


fn clear_bss()
{
    extern "C"
    {
        fn sbss();
        fn ebss();
    }

    (sbss as usize .. ebss as usize).for_each(|a|{
        unsafe { (a as *mut u8).write_volatile(0) }
    });
}

#[no_mangle]
pub fn rust_main() -> !
{
    clear_bss();
    trap_init();
    proc_init();
    run_app();    
}

