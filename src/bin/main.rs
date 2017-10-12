extern crate hfsplus_rescue;

use std::fs::File;
use std::io::{Read, Seek};
use hfsplus_rescue::{FileSystem, FileSlice, ForkData};

fn print_fork_extents<'a, F>(fork: &ForkData<'a, F>) where F: Read + Seek {
    for i in 0..fork.num_extent_descriptors() {
        println!("Extent: {}", i);
        println!("{}", fork.get_extent_descriptor(i));
    }
}

fn main() {
    let device = File::open("./drive.img").unwrap();
    let partition = FileSlice::new(device, 209735680, None).unwrap();
    let fs = FileSystem::new(partition);
    let header = fs.get_volume_header().unwrap();
    println!("Header: {}", header);

    let allocation_fork = header.get_fork_data_allocation();
    println!("Allocation fork: {}", allocation_fork);
    print_fork_extents(&allocation_fork);

    let extents_fork = header.get_fork_data_extents();
    println!("Extents fork: {}", extents_fork);
    print_fork_extents(&extents_fork);

    let catalog_fork = header.get_fork_data_catalog();
    println!("Catalog fork: {}", catalog_fork);
    print_fork_extents(&catalog_fork);

    let attributes_fork = header.get_fork_data_attributes();
    println!("Attributes fork: {}", attributes_fork);
    print_fork_extents(&attributes_fork);

    let startup_fork = header.get_fork_data_startup();
    println!("Startup fork: {}", startup_fork);
    print_fork_extents(&startup_fork);
}
