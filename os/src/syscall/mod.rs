mod proc;
mod fs;
use fs::sys_write;
use crate::task::{exit as sys_exit, suspend as sys_yield, wait, fork, exec};
use crate::timer::get_time_in_ms;
use crate::mm::get_str_from_userspace;

use self::fs::sys_read;

const READ : usize = 63;
const WRITE : usize = 64;
const EXIT : usize = 93;
const YIELD : usize = 124;
const GET_TIME : usize = 169;
const GET_PID : usize = 172;
const FORK : usize = 220;
const EXEC : usize = 221;
const WAIT_PID : usize = 260;

pub fn sys_call(call_num: usize, args: [usize; 3]) -> isize
{
    match call_num
    {
        READ =>
        {
            sys_read(args[0], args[1] as *mut u8, args[2])
        },
        WRITE =>
        {
            sys_write(args[0], args[1] as *const u8, args[2])
        },
        EXIT => 
        {
            sys_exit(args[0] as isize)
        },
        YIELD =>
        {
            sys_yield();
            0
        },
        GET_TIME =>
        {
            get_time_in_ms() as isize  
        },
        WAIT_PID =>
        {
            wait(args[0] as isize, args[1])
        },
        FORK =>
        {
            fork() as isize
        },
        EXEC =>
        {
            let app_name = get_str_from_userspace(args[0]);
            exec(&app_name);
            0
        },
        _ =>
        {
            panic!("did not support syscall {}", call_num)
        },
    }
}
