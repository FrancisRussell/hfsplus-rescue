extern crate hfsplus_rescue;

use std::fs::File;
use hfsplus_rescue::{FileSystem, FileSlice};

fn main() {
    let device = File::open("./drive.img").unwrap();
    let partition = FileSlice::new(device, 209735680, None).unwrap();
    let fs = FileSystem::new(partition);
    let header = fs.get_volume_header().unwrap();
    println!("Header: {}", header);
}
