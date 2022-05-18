use super::task_context::TaskContext;
use crate::config::*;
use crate::sync::up_safe_cell;
use crate::trap::context::Context;
use lazy_static::lazy_static;
use core::arch::global_asm;

global_asm!(include_str!("switch.S"));

#[derive(Copy, Clone)]
pub enum TaskStatus
{
    UnInit,
    Ready,
    Running,
    Exited,
}

#[derive(Copy, Clone)]
pub struct KernelStack
{
    pub data: [u8; KERNEL_STACK_SIZE],
}

impl KernelStack
{
    pub fn stack_top(&self) -> usize
    {
        self.data.as_ptr() as usize + self.data.len()
    }
}

#[derive(Copy, Clone)]
pub struct UserStack
{
    pub data: [u8; USER_STACK_SIZE],
}

impl UserStack 
{
    pub fn stack_top(&self) -> usize
    {
        self.data.as_ptr() as usize + self.data.len()
    }
}
static USER_STACKS : [UserStack; APP_MAX_COUNT] =
[
    UserStack
    {
        data:[0; USER_STACK_SIZE],
    };
    APP_MAX_COUNT
];

static KERNEL_STACKS : [KernelStack; APP_MAX_COUNT] = 
[
    KernelStack
    {
        data:[0; KERNEL_STACK_SIZE],
    };
    APP_MAX_COUNT
];

#[derive(Copy, Clone)]
pub struct Task
{
    pub ctx: TaskContext,
    pub status : TaskStatus,
}
impl Task
{
    pub fn zero_task() -> Self
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
        }
    }
}

struct InnerTaskManager
{
    pub cur: usize,
    pub tasks: [Task; APP_MAX_COUNT],
    pub kernel_ctx: TaskContext,
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
   let apps_ptr = apps as usize as * const usize;
   let app_total = unsafe { apps_ptr.read_volatile() }; 
   
   let mut tasks =
       [
        Task::zero_task();
        APP_MAX_COUNT
       ];

   let mut cur = APP_START;
   for (i,task) in tasks.iter_mut().enumerate()
   {
        init_task(i, task, cur);
        cur += APP_MAXSIZE;
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

pub fn init_task(i:usize, task: &mut Task, user_ra: usize)
{
    extern "C"
    {
        fn __trapret();
    }
    
    let user_stack_top = USER_STACKS[i].stack_top();
    let kernel_stack_top = KERNEL_STACKS[i].stack_top();
    let kernel_ra = __trapret as usize;

    let context_ptr = (kernel_stack_top - core::mem::size_of::<Context>()) as *mut Context;
    unsafe
    {
        *(context_ptr) = Context::new(user_ra, user_stack_top);
    }

    task.ctx.ra = kernel_ra;
    task.ctx.sp = context_ptr as usize;
     
    task.status = TaskStatus::Ready;
    println!("task {} user_ra:{:X}", i, user_ra);
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

    fn get_next(&self) -> core::option::Option<usize>
    {
        let mut inner_tasks = self.tasks.access();
        let mut cur = inner_tasks.cur;
        for _ in 0..self.total
        {
            if let TaskStatus::Ready = inner_tasks.tasks[cur].status
            {
                inner_tasks.cur = cur;
                return core::option::Option::Some(cur);
            }
            cur = (cur + 1) % self.total;
        }
        core::option::Option::None
    }
    
    pub fn run_task(&self) -> !
    {
        loop
        {
            let next_task = self.get_next();
            if let core::option::Option::Some(next_task_index) = next_task
            {
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

pub fn run() -> !
{
    TASK_MANAGER.run_task()
}
