use fs;
use error::HFSPError;
use std::io::{self, Read, Seek, SeekFrom};

pub struct FileSlice<F> {
    offset: u64,
    length: u64,
    file: F,
}

impl<F> FileSlice<F> where F: Seek {
    pub fn new(mut file: F, offset: u64, length: Option<u64>) -> fs::Result<FileSlice<F>> {
        let file_length = file.seek(SeekFrom::End(0))?;
        if file_length < offset {
            return Err(HFSPError::InvalidFileView);
        }
        let length = match length {
            None => file_length - offset,
            Some(length) => if file_length < offset + length {
                return Err(HFSPError::InvalidFileView);
            } else {
                length
            },
        };
        let result = FileSlice {
            offset: offset,
            length: length,
            file: file,
        };
        Ok(result)
    }
}

impl<F> Read for FileSlice<F> where F: Read {
    fn read(&mut self, data: &mut [u8]) -> io::Result<usize> {
        self.file.read(data)
    }
}

impl<F> Seek for FileSlice<F> where F: Seek {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        match pos {
            io::SeekFrom::Start(offset) => {
                self.file.seek(io::SeekFrom::Start(offset + self.offset)).map(|o| o - self.offset)
            },
            io::SeekFrom::Current(offset) => {
                self.file.seek(io::SeekFrom::Current(offset)).map(|o| o - self.offset)
            },
            io::SeekFrom::End(offset) => {
                self.file.seek(io::SeekFrom::End(self.offset as i64 + self.length as i64 + offset)).map(|o| o - self.offset)
            },
        }
    }
}
