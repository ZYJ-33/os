use core::arch::asm;
use crate::{sync::up_safe_cell, trap};
use lazy_static::lazy_static;
use trap::context::Context;

const APP_START : usize = 0x80400000;
const APP_MAX_SIZE : usize = 4096*4;
const APP_COUNT : usize = 5;
const KERNEL_STACK_SIZE : usize = 4096;
const USER_STACK_SIZE : usize = 4096;


struct app_manager
{
    cur : usize,
    apps : [usize; APP_COUNT+1],
}

struct KernelStack
{
    stack : [u8; KERNEL_STACK_SIZE],
}

struct UserStack
{
    stack : [u8; USER_STACK_SIZE],
}

static KERNEL_STACK: KernelStack = KernelStack {
    stack : [0; KERNEL_STACK_SIZE],
};

static USER_STACK: UserStack = UserStack{
    stack : [0; USER_STACK_SIZE],
};

impl KernelStack
{
    fn stack_begin(&self) -> usize
    {
        (self.stack.as_ptr() as usize) + self.stack.len()
    }

    fn push_context(&self, ctx : Context) -> &'static mut Context
    {
        let ctx_ptr = (self.stack_begin() - core::mem::size_of::<Context>()) as *mut Context;
        unsafe
        {
            *ctx_ptr = ctx;
            ctx_ptr.as_mut().unwrap()
        }
    }
}

impl UserStack
{
    fn stack_begin(&self) -> usize
    {
        self.stack.as_ptr() as usize + self.stack.len()
    }
}

lazy_static!
{
static ref APP_MANAGER: up_safe_cell<app_manager> = unsafe
{
    up_safe_cell::new(
        {
            extern "C"
            {
                fn apps();
            }
            let apps_ptr = apps as usize as *const usize;
            let app_slice : &[usize] = core::slice::from_raw_parts(apps_ptr, APP_COUNT+1);
            let mut my_apps : [usize; APP_COUNT+1] = [0; APP_COUNT+1];
            my_apps.copy_from_slice(app_slice);
            app_manager
            {
                cur:0,
                apps: my_apps,
            }
        }
    )
};
}



impl app_manager
{   
    pub fn print_all_apps(&self)
    {
        println!("[kernel] all app");
        for i in 0..APP_COUNT
        {
            println!("app {} begin at {:#x} end at {:#x}", i, self.apps[i], self.apps[i+1]);
        }
    }

    pub unsafe fn load_app(&self)
    {
        if self.cur >= APP_COUNT
        {
            panic!("no app any more");
        }
        println!("loading app {}", self.cur);

        asm!("fence.i");

        if(self.apps[self.cur+1] - self.apps[self.cur] > APP_MAX_SIZE)
        {
            panic!("app {} outsize", self.cur);
        }
        core::slice::from_raw_parts_mut(APP_START as *mut u8, APP_MAX_SIZE).fill(0);
        let src = core::slice::from_raw_parts(self.apps[self.cur] as *const u8,
                                                    self.apps[self.cur+1]-self.apps[self.cur]);
        let dst = core::slice::from_raw_parts_mut(APP_START as *mut u8, self.apps[self.cur+1] - self.apps[self.cur]);
        
        dst.copy_from_slice(src);
        println!("after load prog");
        // self.cur add one?
    }

    pub fn next(&mut self)
    {
        self.cur = self.cur + 1;
    }
}

pub fn init()
{
    print_apps();
}

fn print_apps()
{
    APP_MANAGER.access().print_all_apps();
}

pub fn run_app() -> !
{
    let mut manager = APP_MANAGER.access();
    unsafe
    {
        manager.load_app();
    }
    manager.next();
    drop(manager);
    extern "C"
    {
        fn __trapret(context:usize);
    }

    let fake_ctx = Context::new(APP_START, USER_STACK.stack_begin() as usize);
    println!("after fake ctx");
    println!("{:?}", fake_ctx);
    unsafe
    {
        __trapret(KERNEL_STACK.push_context(fake_ctx) as *const _ as usize);
    }
    panic!("should not reach here");
}
