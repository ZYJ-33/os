use riscv::register::sstatus::{self, Sstatus, SPP};

#[derive(Debug)]
pub struct Context
{
    pub regs : [usize; 32],
    pub sstatus : Sstatus,
    pub sepc : usize,
}

impl Context
{
    pub fn set_sp(&mut self, sp : usize)
    {
        self.regs[2] = sp;
    }

    pub fn new(ret_addr: usize, stack_pointer: usize) -> Self
    {
        let mut ss = sstatus::read();
        ss.set_spp(SPP::User);
        let mut c = Context
        {
            regs : [0; 32],
            sstatus: ss,
            sepc: ret_addr,
        };
        c.set_sp(stack_pointer);
        c
    }
}
