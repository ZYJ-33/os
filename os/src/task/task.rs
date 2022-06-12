extern crate alloc;
use super::task_context::TaskContext;
use crate::{config::*, loader};
use crate::sync::up_safe_cell;
use crate::trap::context::Context;
use crate::trap::before_trapret;
use lazy_static::lazy_static;
use core::arch::global_asm;
use crate::config::PAGE_SIZE;
use crate::mm::{MemorySet, to_prog, PhyPage, PhyAddr, VirPage, KERNEL_SPACE, FrameTracker, alloc};

use crate::loader::get_app;
use alloc::vec::Vec;
use crate::mm::outputentry;

global_asm!(include_str!("switch.S"));

#[derive(Copy, Clone)]
pub enum TaskStatus
{
    UnInit,
    Ready,
    Running,
    Exited,
}


pub struct Task
{
    pub ctx: TaskContext,
    pub status : TaskStatus,
    pub mem: Option<MemorySet>,
    pub trapframe : FrameTracker,
    pub kernelstack_bot : FrameTracker,
    pub userstack_bot : FrameTracker,
}

impl Task
{
    pub fn new_task() -> Self
    {
        Task 
        {
            ctx: TaskContext 
            { 
                callee_save: [0;12], 
                sp: 0, 
                ra: 0,
            },
            status: TaskStatus::UnInit,
            mem: None,
            trapframe: alloc().unwrap(),
            kernelstack_bot: alloc().unwrap(),
            userstack_bot: alloc().unwrap(),
        }
    }
    pub fn get_ctx(&mut self) -> &mut Context
    {
        let ptr:*mut Context = (usize::from(self.trapframe.ppn()) << PAGE_BITS) as *mut Context;
        unsafe
        {
            ptr.as_mut().unwrap()
        }
    }
}

struct InnerTaskManager
{
    pub cur: usize,
    pub tasks: Vec<Task>,
    pub kernel_ctx: TaskContext,
}

impl InnerTaskManager
{
    pub fn get_cur_task(&self) -> &Task
    {
        self.tasks.get(self.cur).unwrap()
    }

    pub fn get_cur_task_mut(&mut self) -> &mut Task
    {
        self.tasks.get_mut(self.cur).unwrap()
    }
}

pub struct TaskManager
{
    total: usize,
    tasks: up_safe_cell<InnerTaskManager>,
}

lazy_static!
{
pub static ref TASK_MANAGER : TaskManager = 
{
   extern "C"
   {
        fn apps();
   }
   let mut tasks : Vec<Task> = Vec::new();
   let apps_ptr = apps as usize as * const usize;
   let app_total = unsafe { apps_ptr.read_volatile()}; 
   for i in 0 .. app_total
   {
       tasks.push(Task::new_task());
       init_task(i, tasks.get_mut(i).unwrap())
   }
   
   let inner_task_manager = InnerTaskManager 
   {
       cur: 0, 
       tasks: tasks,
       kernel_ctx: TaskContext 
       { 
           callee_save: [0;12], 
           sp: 0, 
           ra: 0,
       }
   };
    TaskManager
    {
        total : app_total,
        tasks : unsafe {up_safe_cell::new(inner_task_manager)},
    }
};
}

extern "C"
{
    fn switch(saved: *mut TaskContext, loaded: *const TaskContext);
}

pub fn init_task(i:usize, task: &mut Task)
{
    let kernel_ra = before_trapret as usize;
    let elf = loader::get_app(i);
    let entry_point: usize;
    let mut mem_set: MemorySet;
    (entry_point, mem_set) = to_prog(elf);

    let user_stack_va = mem_set.map_userstack(task.userstack_bot.ppn());
    mem_set.map_trapframe(task.trapframe.ppn());
    
    let context_ptr : *mut Context = usize::from(PhyAddr::from(task.trapframe.ppn())) as *mut Context;
    let kernel_stack_top = usize::from(PhyAddr::from(task.kernelstack_bot.ppn())) + PAGE_SIZE;
    let ctx = Context::new(entry_point, user_stack_va, KERNEL_SPACE.access().root_satp(), kernel_stack_top);
    unsafe
    {
        *(context_ptr) = ctx;
    }

    task.ctx.ra = kernel_ra;
    task.ctx.sp = kernel_stack_top as usize;
    task.mem = Some(mem_set);
    task.status = TaskStatus::Ready;
}

impl TaskManager
{
    pub fn suspend_cur_task(&self)
    {
        let mut tasks = self.tasks.access();
        let cur = tasks.cur;

        tasks.tasks[cur].status = TaskStatus::Ready;
        tasks.cur = (cur + 1) % self.total;
        let saved = &mut tasks.tasks[cur].ctx as *mut TaskContext;
        let loaded = &tasks.kernel_ctx as *const TaskContext;
        drop(tasks);

        unsafe
        {
            switch(saved, loaded)
        }
    }

    pub fn get_cur_task_ctx(&self) -> usize
    {
        let mut tasks = self.tasks.access();
        let cur = tasks.cur;
        (usize::from(tasks.tasks[cur].trapframe.ppn())) << PAGE_BITS
    }

    pub fn exit_cur_task(&self) -> !
    {
        let mut tasks = self.tasks.access();
        let cur = tasks.cur;
        
        tasks.tasks[cur].status = TaskStatus::Exited;
        let saved = &mut tasks.tasks[cur].ctx as *mut TaskContext;
        let loaded = &tasks.kernel_ctx as *const TaskContext;
        drop(tasks);
        
        unsafe
        {
            switch(saved , loaded)
        }
        panic!("should not reach here in exit");
    }

    fn get_next(&self) -> Option<usize>
    {
        let mut inner_tasks = self.tasks.access();
        let mut cur = inner_tasks.cur;
        for _ in 0..self.total
        {
            if let TaskStatus::Ready = inner_tasks.tasks[cur].status
            {
                inner_tasks.cur = cur;
                return Some(cur);
            }
            cur = (cur + 1) % self.total;
        }
        None
    }
    
    pub fn run_task(&self) -> !
    {
        loop
        {
            let next_task = self.get_next();
            if let core::option::Option::Some(next_task_index) = next_task
            {
                println!("get task {}", next_task_index);
                let mut tasks = self.tasks.access();
                let saved = &mut tasks.kernel_ctx  as *mut TaskContext;
                let loaded = &(tasks.tasks[next_task_index].ctx)  as *const TaskContext;
                drop(tasks);
                unsafe
                {
                    switch(saved, loaded);
                }
            }
            else
            {
                panic!("no more task\n");
            }
        }
    }
}

pub fn exit() -> !
{
        TASK_MANAGER.exit_cur_task();  
}

pub fn suspend()
{
        TASK_MANAGER.suspend_cur_task();
}

pub fn get_current_task_ctx() -> usize
{
        TASK_MANAGER.get_cur_task_ctx()
}

pub fn get_current_task_satp() -> usize
{
        let tasks = TASK_MANAGER.tasks.access();
        ((tasks.tasks.get(tasks.cur).unwrap()).mem).as_ref().unwrap().root_satp()
}

pub fn get_current_task_pgt() -> PhyPage
{
        let tasks = TASK_MANAGER.tasks.access();
        PhyPage::from((tasks.tasks.get(tasks.cur).unwrap()).mem.as_ref().unwrap().root_satp()
            & 0x0fffffffffffffff)
}

pub fn run() -> !
{
        TASK_MANAGER.run_task()
}
