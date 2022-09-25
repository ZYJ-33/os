extern crate alloc;
use super::task_context::TaskContext;
use crate::loader::get_app_by_name;
use crate::{config::*, loader};
use crate::sync::up_safe_cell;
use crate::trap::context::Context;
use crate::trap::before_trapret;
use alloc::collections::VecDeque;
use alloc::vec::{Vec};
use lazy_static::lazy_static;
use core::arch::global_asm;
use crate::config::PAGE_SIZE;
use crate::mm::{MemorySet, to_prog, PhyPage, PhyAddr, VirPage, KERNEL_SPACE, FrameTracker, alloc, copy_page, VirAddr};
use super::pid::{Pid, alloc as pid_alloc};
use alloc::sync::{Arc, Weak};
use crate::loader::get_index_by_name;


global_asm!(include_str!("switch.S"));

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum TaskStatus
{
    UnInit,
    Ready,
    Running,
    Zombie,
    Exited,
}

pub struct TaskInner
{
    pub ctx: TaskContext,
    pub status : TaskStatus,
    pub mem: Option<MemorySet>,
    pub trapframe : FrameTracker,
    pub kernelstack_bot : FrameTracker,
    pub userstack_bot : FrameTracker,
    pub parent: Option<Weak<Task>>,
    pub childrens: Vec<Arc<Task>>,
    pub exit_code: isize,
}

pub struct Task
{
   pub task_inner: up_safe_cell<TaskInner>,
   pub pid: Pid,
}

impl Task
{
    pub fn new_task() -> Self
    {
        let task_inner = 
        unsafe
        {
            up_safe_cell::new
            (
                TaskInner
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
                    parent: None,
                    childrens: Vec::new(),
                    exit_code: 0,
                }
            )
        };
        Task 
        {
            task_inner,
            pid: pid_alloc(),
        }
    }

    pub fn fork_other(self: &Arc<Task>) -> Arc<Task>
    {
        let my_task_inner = self.task_inner.access();

        let trapframe = alloc().unwrap();
        let kernelstack_bot = alloc().unwrap();
        let userstack_bot = alloc().unwrap();
        // copy userstack
        copy_page(userstack_bot.ppn, my_task_inner.userstack_bot.ppn);

        // init trapframe
        let src_ctx_ptr = usize::from(PhyAddr::from(my_task_inner.trapframe.ppn)) as *const Context;
        let dst_ctx_ptr = usize::from(PhyAddr::from(trapframe.ppn)) as *mut Context;
       unsafe
       {
            let dst_ctx = dst_ctx_ptr.as_mut().unwrap();
            let src_ctx = src_ctx_ptr.as_ref().unwrap();
            *dst_ctx = *src_ctx;
            dst_ctx.regs[10] = 0;
            dst_ctx.kernel_stack = usize::from(PhyAddr::from(kernelstack_bot.ppn)) + PAGE_SIZE;
       }

        let mut mem_set = my_task_inner.mem.as_ref().unwrap().fork_one();
        mem_set.map_trampoline();
        mem_set.map_userstack(userstack_bot.ppn);
        mem_set.map_trapframe(trapframe.ppn);
        // initialze kernelstack userstack trapframe correctly

        let new_task_ctx = TaskContext::new(
            usize::from(PhyAddr::from(kernelstack_bot.ppn)) + PAGE_SIZE,
            before_trapret as usize    
        );

        let task_inner = 
        unsafe
        {
            up_safe_cell::new
            (
                TaskInner
                {
                    ctx: new_task_ctx,
                    status: my_task_inner.status,
                    mem: Some(mem_set),
                    trapframe,
                    kernelstack_bot,
                    userstack_bot,
                    parent:Some(Arc::downgrade(self)),
                    childrens: Vec::new(),
                    exit_code: 0,
                }
            )
        };

        Arc::new(
        Task
        {
            task_inner,
            pid: pid_alloc(),
        }
        )
    }

    pub fn exec(&self, app_name: &str)
    {
        // set new memset trapframe
        let (entry_point, memset) = to_prog(get_app_by_name(app_name));
        let mut task_inner = self.task_inner.access();
        let old_memset = task_inner.mem.take();
        
        let trap_context_ptr = usize::from(PhyAddr::from(task_inner.trapframe.ppn())) as *mut Context;
        let user_stack_top = usize::from(PhyAddr::from(task_inner.userstack_bot.ppn())) + PAGE_SIZE;
        
        unsafe
        {
            let mut trap_context = trap_context_ptr.as_mut().unwrap();
            trap_context.regs = [0;32];
            trap_context.sepc = entry_point;
            trap_context.set_sp(user_stack_top);
        }
        task_inner.mem = Some(memset); 
        drop(old_memset);
    }

    pub fn get_ctx(&mut self) -> &mut Context
    {
        let inner_task = self.task_inner.access();
        let ptr:*mut Context = (usize::from(inner_task.trapframe.ppn()) << PAGE_BITS) as *mut Context;
        unsafe
        {
            ptr.as_mut().unwrap()
        }
    }
}

struct InnerTaskManager
{
    pub cur: Option<Arc<Task>>,
    pub tasks: VecDeque<Arc<Task>>,
    pub kernel_ctx: TaskContext,
}

impl InnerTaskManager
{
    pub fn get_cur_task(&self) -> Option<Arc<Task>>
    {
        self.cur.as_ref().map(Arc::clone)
    }

    pub fn take_cur_task(&mut self) -> Option<Arc<Task>>
    {
        self.cur.take()
    }
}

pub struct TaskManager
{
    total: usize,
    init_task: up_safe_cell<Arc<Task>>,
    tasks: up_safe_cell<InnerTaskManager>,
}

lazy_static!
{
pub static ref TASK_MANAGER : up_safe_cell<TaskManager> =
unsafe
{
    up_safe_cell::new(
        {
           extern "C"
           {
                fn apps();
           }
           let mut tasks : VecDeque<Arc<Task>> = VecDeque::new();
           let apps_ptr = apps as usize as * const usize;
           let app_total = unsafe { apps_ptr.read_volatile()}; 

           let mut init = Task::new_task();
           init_task(get_index_by_name("init"), &mut init);
           let tmp = Arc::new(init);
           tasks.push_front(tmp.clone());

           let inner_task_manager = InnerTaskManager 
           {
               cur: None, 
               tasks: tasks,
               kernel_ctx: TaskContext::new(0, 0),
           };

            TaskManager
            {
                total : app_total,
                init_task: unsafe {up_safe_cell::new(tmp)},
                tasks : unsafe {up_safe_cell::new(inner_task_manager)},
            }
        }
)};
}

extern "C"
{
    fn switch(saved: *mut TaskContext, loaded: *const TaskContext);
}

pub fn init_task(i:usize, task_out: &mut Task)
{
    let mut task = task_out.task_inner.access();
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
    fn clean_up_exited_task(&self, other:Arc<Task>)
    {
        let inner = other.task_inner.access();
        assert!(inner.status == TaskStatus::Zombie);
        //TODO: clean memory set of exited process
        assert!(Arc::strong_count(&other) == 1);
        drop(inner);
        drop(other);
    }

    pub fn wait(&self, pid: isize, user_addr: usize) -> isize
    {
        let task_manager = self.tasks.access();
        let cur_task = task_manager.get_cur_task().unwrap();
        drop(task_manager);

        let mut task_inner = cur_task.task_inner.access();

        let index__zombie = task_inner.childrens.iter().enumerate().find(
            |(_, task)|
            {
                task.task_inner.access().status == TaskStatus::Zombie
            &&  (pid == -1 || task.pid.0 == pid)
            }
        );

        if let Some((index, _)) = index__zombie
        {
            let zombie = task_inner.childrens.remove(index);
            let exit_code = zombie.task_inner.access().exit_code;
            let pid = zombie.pid.0;
            self.clean_up_exited_task(zombie);
            *(task_inner.mem.as_ref().unwrap()
                .pgt.translate_va(VirAddr::from(user_addr)).unwrap()
                .get_mut()) = exit_code;
            return pid
        }
        -1
    }

    pub fn suspend_cur_task(&self)
    {
        let mut task_manager = self.tasks.access();
        let cur_task = task_manager.take_cur_task();

        if let Some(task) = cur_task
        {
            let mut task_inner = task.task_inner.access();

            let saved = &mut (task_inner.ctx) as *mut TaskContext;
            let loaded =&(task_manager.kernel_ctx) as *const TaskContext;

            drop(task_inner);
            (task_manager).tasks.push_back(
                task
            );

            drop(task_manager);
            unsafe
            {
                switch(saved, loaded)
            }
        }
        else
        {
            panic!("suspend_cur_task: did not have current task running");
        }
    }

    fn index_of(&self, target:& Arc<Task>) -> usize
    {
        let tasks = &self.tasks.access().tasks;
        let (index, _) = tasks.iter().enumerate().find(|(i, task)| task.pid == target.pid).unwrap();
        index
    }

    pub fn get_cur_task_ctx(&self) -> usize
    {
        let tasks = self.tasks.access();
        if let Some(task) = tasks.get_cur_task()
        {
            usize::from( task.task_inner.access().trapframe.ppn()) << PAGE_BITS
        }
        else
        {
            panic!("get_cur_task_ctx: did not have current task running");
        }
    }
    
    pub fn run_task(&self) -> !
    {
        loop
        {
            let mut inner_manager = self.tasks.access();

            let selected = inner_manager.tasks.pop_front().unwrap();
            let selected_inner = selected.task_inner.access();

            let loaded = &(selected_inner.ctx) as *const TaskContext;
            let saved = &mut (inner_manager.kernel_ctx) as *mut TaskContext;

            drop(selected_inner);
            (inner_manager).cur = Some(selected);
            
            drop(inner_manager);


            unsafe
            {
                switch(saved, loaded);
            }
        }
    }

    pub fn exit_cur_task(&self, code:isize) -> !
    {
        let cur_task = self.tasks.access().take_cur_task().unwrap();
        let mut task_inner = cur_task.task_inner.access();

        task_inner.status = TaskStatus::Zombie;
        task_inner.exit_code = code;
        
        let new_parent = self.init_task.access();

        for i in (0..task_inner.childrens.len())
        {
            let child = &task_inner.childrens[i];
            child.task_inner.access().parent = Some(Arc::downgrade(&*new_parent));
            new_parent.task_inner.access().childrens.push(child.clone());
        }

        let loaded = &(self.tasks.access().kernel_ctx) as *const TaskContext;
        let saved = &mut task_inner.ctx as *mut TaskContext;

        task_inner.childrens.clear();
        
        drop(new_parent);
        drop(task_inner);
        drop(cur_task);

        unsafe
        {
            switch(saved, loaded)
        };
        panic!("should reach here in exit\n");
    }
}

pub fn exit(exit_code : isize) -> !
{
        TASK_MANAGER.access().exit_cur_task(exit_code);
}

pub fn suspend()
{
        TASK_MANAGER.access().suspend_cur_task();
}

pub fn fork() -> usize
{
    let manager = TASK_MANAGER.access();

    let mut inner_manager = manager.tasks.access();
    let cur = inner_manager.get_cur_task().unwrap();

    let new_task = cur.fork_other();

    new_task.task_inner.access().parent = Some(Arc::downgrade(&cur));
    cur.task_inner.access().childrens.push(new_task.clone());
    
    let pid = new_task.pid.0 as usize;
    inner_manager.tasks.push_back(new_task);
    
    pid
}

pub fn exec(app_name: &str)
{
    return TASK_MANAGER.access().tasks.access().get_cur_task().unwrap().exec(app_name);        
}

pub fn wait(pid: isize, user_addr: usize) -> isize
{
    TASK_MANAGER.access().wait(pid, user_addr)
}

pub fn get_current_task_ctx() -> usize
{
        TASK_MANAGER.access().get_cur_task_ctx()
}

pub fn get_current_task_satp() -> usize
{
        let cur = TASK_MANAGER.access().tasks.access().get_cur_task();
        if let Some(task) = cur
        {
            task.task_inner.access().mem.as_ref().unwrap().root_satp()
        }
        else
        {
            panic!("did not have current task in get_current_task_satp");
        }
}

pub fn get_current_task_pgt() -> PhyPage
{
        let cur = TASK_MANAGER.access().tasks.access().get_cur_task();
        if let Some(task) = cur
        {
            task.task_inner.access().mem.as_ref().unwrap().pgt.root
        }
        else
        {
            panic!("did not have current task in get_current_task_satp");
        }
}

pub fn run() -> !
{
        TASK_MANAGER.access().run_task()
}
