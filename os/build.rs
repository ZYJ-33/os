use std::fs::{File, read_dir};


fn main()
{
    let entitys = read_dir("../user/src/bin/").unwrap();

    let entitys : Vec<_> = 
    entitys.into_iter().map(|entity|{
        let mut file_name = entity.unwrap().file_name().into_string().unwrap();
        file_name.drain(file_name.find(".").unwrap()..file_name.len());
        file_name
    }).collect();

    entitys.into_iter().for_each(|entity|{println!("{}", entity)});
    
}

