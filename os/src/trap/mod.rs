pub mod context;
use riscv::register::
{ 
    scause::
    {
        self,
        Exception, 
        Trap,
        Scause, Interrupt,
    },
    stval, stvec, sstatus,
};

use crate::syscall::sys_call;
use crate::task::{exit, suspend};
use crate::timer::set_next_timer_intr_in_ms;
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
    /*
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
    */


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
            exit();
        }
        Trap::Exception(Exception::IllegalInstruction) =>
        {
            println!("illegal inst");
            exit();
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) =>
        {
            set_next_timer_intr_in_ms(10);
            println!("timer interupt happen");
            suspend();
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
