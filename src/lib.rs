#![feature(concat_idents)]
extern crate byteorder;

mod error;
mod file_slice;
mod filesystem;

pub mod fs;

pub use filesystem::FileSystem;
pub use error::HFSPError;
pub use file_slice::FileSlice;

