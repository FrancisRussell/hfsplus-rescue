extern crate byteorder;
extern crate num;

mod error;
mod file_slice;
mod filesystem;

pub mod fs;

pub use filesystem::{FileSystem, VolumeHeader, ForkData};
pub use error::HFSPError;
pub use file_slice::FileSlice;

