use fs;
use error::HFSPError;
use std::io::{Read, Seek, SeekFrom};
use std::sync::Mutex;
use std::fmt::{self, Display, Formatter};
use byteorder::{BigEndian, ReadBytesExt};

#[derive(Debug)]
pub struct FileSystem<F> {
    file: Mutex<F>,
}

impl<F> FileSystem<F> where F: Read + Seek {
    pub fn new(file: F) -> FileSystem<F> {
        FileSystem {
            file: Mutex::new(file),
        }
    }

    pub fn get_volume_header<'a>(&'a self) -> fs::Result<VolumeHeader<'a, F>> {
        let result = VolumeHeader::new(self, 1024);
        result.validate()?;
        Ok(result)
    }

    fn validate_bytes(&self, offset: u64, bytes: &[u8]) -> fs::Result<()> {
        let mut file = self.file.lock().unwrap();
        file.seek(SeekFrom::Start(offset))?;
        let mut data = vec![0; bytes.len()];
        file.read_exact(&mut data[..])?;
        for (x, y) in bytes.iter().zip(data.iter()) {
            if x != y {
                return Err(HFSPError::InvalidVolumeHeader);
            }
        }
        Ok(())
    }

    fn read_u16(&self, offset: u64) -> fs::Result<u16> {
        let mut file = self.file.lock().unwrap();
        file.seek(SeekFrom::Start(offset))?;
        let result = file.read_u16::<BigEndian>()?;
        Ok(result)
    }

    fn read_u32(&self, offset: u64) -> fs::Result<u32> {
        let mut file = self.file.lock().unwrap();
        file.seek(SeekFrom::Start(offset))?;
        let result = file.read_u32::<BigEndian>()?;
        Ok(result)
    }
}

#[derive(Debug)]
pub struct VolumeHeader<'a, F> where F: 'a {
    parent: &'a FileSystem<F>,
    offset: u64,
}

impl<'a, F> VolumeHeader<'a, F> where F: Read + Seek {
    fn new(parent: &'a FileSystem<F>, offset: u64) -> VolumeHeader<'a, F> {
        VolumeHeader {
            parent: parent,
            offset: offset,
        }
    }

    fn validate(&self) -> fs::Result<()> {
        self.parent.validate_bytes(self.offset, b"H+")
    }

    pub fn get_version(&self) -> fs::Result<u16> {
        self.parent.read_u16(self.offset + 2)
    }

    pub fn get_file_count(&self) -> fs::Result<u32> {
        self.parent.read_u32(self.offset + 32)
    }

    pub fn get_folder_count(&self) -> fs::Result<u32> {
        self.parent.read_u32(self.offset + 36)
    }

    pub fn get_block_size(&self) -> fs::Result<u32> {
        self.parent.read_u32(self.offset + 40)
    }

    pub fn get_total_blocks(&self) -> fs::Result<u32> {
        self.parent.read_u32(self.offset + 44)
    }

    pub fn get_free_blocks(&self) -> fs::Result<u32> {
        self.parent.read_u32(self.offset + 48)
    }
}

impl<'a, F> Display for VolumeHeader<'a, F> where F: Read + Seek {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        writeln!(fmt, "Version: {:?}", self.get_version())?;
        writeln!(fmt, "Folder count: {:?}", self.get_folder_count())?;
        writeln!(fmt, "File count: {:?}", self.get_file_count())?;
        writeln!(fmt, "Block size: {:?}", self.get_block_size())?;
        writeln!(fmt, "Total blocks: {:?}", self.get_total_blocks())?;
        writeln!(fmt, "Free blocks: {:?}", self.get_free_blocks())?;
        Ok(())
    }
}
