mod fs;
use fs::sys_write;
use crate::batch::run_app;

const WRITE : usize = 64;
const EXIT : usize = 93;

pub fn sys_call(call_num: usize, args: [usize ; 3]) -> isize
{
    match call_num
    {
        WRITE =>
        {
            sys_write(args[0], args[1] as *const u8, args[2])
        },
        EXIT => 
        {
            run_app();
            panic!("should not reach here in exit call");
        },
        _ =>{
            panic!("did not support syscall {}", call_num)
        },
    }
        
}
