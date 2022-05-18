use crate::config::*;

extern "C"
{
    fn apps();
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

fn load_app(i:usize, dst_addr:usize)
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
     
    let src = 
    unsafe
    {
        core::slice::from_raw_parts(app_start as *const u8, app_len)
    };

    let dst =
    unsafe
    {
        core::slice::from_raw_parts_mut(dst_addr as *mut u8, app_len)
    };

    unsafe
    {
        core::slice::from_raw_parts_mut(dst_addr as *mut u8, APP_MAXSIZE).fill(0);
    };
    dst.copy_from_slice(src);
    println!("app {} begin at {} with len {}",i, dst.as_ptr() as usize, dst.len());
}

fn load_all_app(count: usize)
{
    let mut cur = APP_START;
    for i in (0..count)
    {
        unsafe
        {
            load_app(i, cur);
        }
        cur += APP_MAXSIZE;
    }
}

pub fn init()
{
    load_all_app(read_i_word(0));
}
