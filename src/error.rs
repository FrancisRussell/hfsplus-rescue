use std::convert;
use std::error;
use std::fmt;
use std::io;

#[derive(Debug)]
pub enum HFSPError {
    IOError(io::Error),
    InvalidVolumeHeader,
    InvalidFileView,
}

impl fmt::Display for HFSPError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use std::error::Error;
        match *self {
            _ => write!(f, "{}", self.description()),
        }
    }
}

impl error::Error for HFSPError {
    fn description(&self) -> &str {
        match *self {
            HFSPError::IOError(_) => &"IO Error",
            HFSPError::InvalidVolumeHeader => &"Invalid Volume Header",
            HFSPError::InvalidFileView => &"Invalid partition offset or length",
        }
    }
}

impl convert::From<io::Error> for HFSPError {
    fn from(error: io::Error) -> Self {
        HFSPError::IOError(error)
    }
}
