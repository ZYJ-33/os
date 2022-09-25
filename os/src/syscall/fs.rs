extern crate alloc;
use alloc::vec::Vec;
use crate::sbi::console_getchar;
use crate::mm::get_arr_from_userspace;
use crate::task::{get_current_task_satp, suspend};

const STDOUT : usize = 1;
const STDIN : usize = 0;

pub fn sys_read(fd : usize, buf:*mut u8, len: usize) -> isize
{
    match fd
    {
        STDIN =>
        {
            let satp = get_current_task_satp();
            let mut segments : Vec<&'static mut [u8]> = get_arr_from_userspace(satp, buf as usize, len);
            let mut _len = len;
            let mut cur_seg_index = 0;
            
            while cur_seg_index < segments.len()
            {
                let mut i = 0;
                let cur_seg = &mut segments[cur_seg_index];
                while i < cur_seg.len()
                {
                    let c = console_getchar();
                    if c == 0
                    {
                        suspend();
                        continue;
                    }
                    else
                    {
                        cur_seg[i] = c as u8;
                        i += 1;
                    }
                }
                cur_seg_index += 1;
            }
            1
        }
        _ =>
        {
            panic!("unsupport fd {} in sys_read", fd);
        }
    }
}

pub fn sys_write(fd : usize, buf: * const u8, len: usize) -> isize
{
    match fd
    {
        STDOUT =>
        {
            let satp  =get_current_task_satp();
            let slices = get_arr_from_userspace(satp, buf as usize, len);
            for slice in slices
            {
                let str = core::str::from_utf8(slice).unwrap();
                print!("{}", str);
            }
            len as isize
        },
        _ =>
        {
            println!("not support fd yet");
            -1
        },
    }
}
