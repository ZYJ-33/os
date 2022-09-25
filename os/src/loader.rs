extern crate alloc;
use alloc::vec::Vec;
use lazy_static::*;
use crate::config::*;

extern "C"
{
    fn apps();
    fn app_names();
}

lazy_static!
{
    static ref APP_NAMES: Vec<&'static str> = get_app_names();
}

pub fn get_app_names() -> Vec<&'static str>
{
    let count = get_app_count();
    let mut v = Vec::new();
    let mut cur = app_names as *const u8;
    let mut start = cur;
    for i in 0..count
    {
       loop
       {
            let char = unsafe {cur.read_volatile()} ;
            if(char == b'\0')
            {
                let app_name = unsafe{ core::slice::from_raw_parts(start, cur as usize - start as usize) };
                v.push( core::str::from_utf8(app_name).unwrap() );
                cur = unsafe {cur.add(1)};
                start = cur;
                break;
            }
            else
            {
                cur = unsafe {cur.add(1)};
            }
       }
    }
    v
}

pub fn read_i_word(i: usize) -> usize
{
    let base_addr = apps as usize as *const usize;
    unsafe
    {
        base_addr.add(i).read_volatile()
    }
}

pub fn get_app_count() -> usize
{
    unsafe
    {
        read_i_word(0)
    }
}

pub fn get_index_by_name(target: &str) -> usize
{
    let res = APP_NAMES.iter().enumerate().find(|(_, name)| **name == target);
    if let Some((index, _)) = res
    {
        index
    }
    else
    {
        panic!("can't find app {} in get_index_by_name", target);
    }
}

pub fn get_app_by_name(target: &str) -> &'static [u8]
{
    let index = get_index_by_name(target);
    get_app(index)
}

pub fn get_app(i:usize) -> &'static [u8]
{
    let count = get_app_count();
    if i >= count
    {
        println!("{} th app not exist", i);
        panic!("in load");
    }
    let app_start =
    unsafe
    {
        read_i_word(i+1)
    };
    let app_end = 
    unsafe
    {
        read_i_word(i+2)
    };
    let app_len = app_end - app_start;
     
    unsafe 
    {
        core::slice::from_raw_parts(app_start as *const u8, app_len)
    } 
}
