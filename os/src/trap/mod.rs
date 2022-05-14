pub mod context;
use riscv::register::{ 
    scause::{self, Exception, Trap, Scause},
    stval, stvec, sstatus,
    };
use crate::{run_app, sys_call};
use context::Context;
use core::arch::global_asm;

global_asm!(include_str!("trap.S"));

pub fn init()
{
    extern "C"
    {
        fn __usertrap();
    }
    unsafe
    {
        stvec::write(__usertrap as usize, stvec::TrapMode::Direct);
    }
}

#[no_mangle]
fn trap_handler(ctx :&mut Context) -> &mut Context
{
    let val = stval::read();
    let scause = scause::read();
    let mut ss = sstatus::read();
    match ss.spp()
    {
        sstatus::SPP::User =>
        {
            ()
        }
        sstatus::SPP::Supervisor =>
        {
            match scause.cause()
            {
                Trap::Exception(Exception::IllegalInstruction) =>
                {
                    println!("illegal instr");
                }
                _ =>
                {
                    panic!("unknown")
                }
            }
        }
    }

    match scause.cause() 
    {
        Trap::Exception(Exception::UserEnvCall) =>
        {
            ctx.sepc += 4;
            ctx.regs[10] = sys_call(ctx.regs[17], [ctx.regs[10], ctx.regs[11], ctx.regs[12]]) as usize;
        }
        Trap::Exception(Exception::StoreFault) =>
        {
            println!("store fault");
            run_app();
        }
        Trap::Exception(Exception::IllegalInstruction) =>
        {
            println!("illegal inst");
            run_app();
        }
        _  =>
        {
            panic!(
                "unsupport trap {:?} , stval = {:#x}!",
                scause.cause(),
                val
                );
        }
    }

    ctx
}
