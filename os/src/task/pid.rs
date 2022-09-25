extern crate alloc;
use alloc::collections::LinkedList;
use lazy_static::lazy_static;
use crate::sync::up_safe_cell;

lazy_static!
{
    pub static ref PID_MANAGER: up_safe_cell<PidManager> = unsafe { up_safe_cell::new(PidManager::new()) };
}

pub struct PidManager
{
    cur: isize,
    saved: LinkedList<isize>,
}

impl PidManager
{
    pub fn new() -> Self
    {
        PidManager
        {
            cur: 0,
            saved: LinkedList::new(),
        }
    }

    pub fn alloc(&mut self) -> isize
    {
        if let Some(pid) = self.saved.pop_front()
        {
            pid
        }
        else
        {
           let ret = self.cur;
           self.cur += 1;
           ret
        }
    }

    pub fn dealloc(&mut self, pid: isize)
    {
        assert!(pid < self.cur);
        assert!(
                !self.saved.iter().any(|saved| *saved == pid),
                "pid {} has already been alloced\n",
                pid
            );
        self.saved.push_back(pid)
    }
}

#[derive(PartialEq, Eq)]
pub struct Pid(pub isize);

impl Drop for Pid
{
    fn drop(&mut self) 
    {
        PID_MANAGER.access().dealloc(self.0)
    }
}

pub fn alloc() -> Pid
{
    Pid(PID_MANAGER.access().alloc())
}
