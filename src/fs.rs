use error::HFSPError;
use std::result;

pub type Result<T> = result::Result<T, HFSPError>;
