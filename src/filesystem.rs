use fs;
use error::HFSPError;
use std::io::{Read, Seek, SeekFrom};
use std::sync::Mutex;
use std::fmt::{self, Display, Formatter};
use std::mem;
use std::slice;
use num;

const OFFSET_VOLUME_HEADER: u64 = 1024;
const OFFSET_VOLUME_HEADER_FORKS: u64 = 112;
const OFFSET_FORK_DATA_EXTENT_RECORD: u64 = 16;
const SIZE_EXTENT_DESCRIPTOR: u64 = 8;
const SIZE_EXTENT_RECORD: u64 = SIZE_EXTENT_DESCRIPTOR * 8;
const SIZE_FORK_DATA: u64 = 16 + SIZE_EXTENT_RECORD;

#[derive(Debug)]
pub struct FileSystem<F> {
    file: Mutex<F>,
}

pub trait Structure<F> {
    fn get_offset(&self) -> u64;
    fn get_filesystem(&self) -> &FileSystem<F>;

    fn read_number<T: num::PrimInt>(&self, offset: usize) -> fs::Result<T> where F: Read + Seek {
        let mut result: T = T::zero();
        let ptr = &mut result as *mut T as *mut u8;
        let length = mem::size_of::<T>();
        let mut buffer = unsafe { slice::from_raw_parts_mut(ptr, length) };
        {
            let mut file = self.get_filesystem().file.lock().unwrap();
            file.seek(SeekFrom::Start(self.get_offset() + offset as u64))?;
            file.read_exact(&mut buffer[..])?;
        }
        let result = num::PrimInt::from_be(result);
        Ok(result)
    }
}

impl<F> FileSystem<F> where F: Read + Seek {
    pub fn new(file: F) -> FileSystem<F> {
        FileSystem {
            file: Mutex::new(file),
        }
    }

    pub fn get_volume_header<'a>(&'a self) -> fs::Result<VolumeHeader<'a, F>> {
        let result = VolumeHeader::new(self, OFFSET_VOLUME_HEADER);
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
}

#[derive(Debug)]
pub struct VolumeHeader<'a, F> where F: 'a {
    parent: &'a FileSystem<F>,
    offset: u64,
}

impl<'a, F> Structure<F> for VolumeHeader<'a, F> where F: 'a {
    fn get_offset(&self) -> u64 {
        self.offset
    }

    fn get_filesystem(&self) -> &FileSystem<F> {
        self.parent
    }
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
        self.read_number(2)
    }

    pub fn get_file_count(&self) -> fs::Result<u32> {
        self.read_number(32)
    }

    pub fn get_folder_count(&self) -> fs::Result<u32> {
        self.read_number(36)
    }

    pub fn get_block_size(&self) -> fs::Result<u32> {
        self.read_number(40)
    }

    pub fn get_total_blocks(&self) -> fs::Result<u32> {
        self.read_number(44)
    }

    pub fn get_free_blocks(&self) -> fs::Result<u32> {
        self.read_number(48)
    }

    pub fn get_fork_data_allocation(&self) -> ForkData<'a, F> {
        ForkData::new(self.parent, self.offset + OFFSET_VOLUME_HEADER_FORKS)
    }

    pub fn get_fork_data_extents(&self) -> ForkData<'a, F> {
        ForkData::new(self.parent, self.offset + OFFSET_VOLUME_HEADER_FORKS + SIZE_FORK_DATA)
    }

    pub fn get_fork_data_catalog(&self) -> ForkData<'a, F> {
        ForkData::new(self.parent, self.offset + OFFSET_VOLUME_HEADER_FORKS + SIZE_FORK_DATA * 2)
    }

    pub fn get_fork_data_attributes(&self) -> ForkData<'a, F> {
        ForkData::new(self.parent, self.offset + OFFSET_VOLUME_HEADER_FORKS + SIZE_FORK_DATA * 3)
    }

    pub fn get_fork_data_startup(&self) -> ForkData<'a, F> {
        ForkData::new(self.parent, self.offset + OFFSET_VOLUME_HEADER_FORKS + SIZE_FORK_DATA * 4)
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

#[derive(Debug)]
pub struct ForkData<'a, F> where F: 'a {
    parent: &'a FileSystem<F>,
    offset: u64,
}

impl<'a, F> Structure<F> for ForkData<'a, F> where F: 'a {
    fn get_offset(&self) -> u64 {
        self.offset
    }

    fn get_filesystem(&self) -> &FileSystem<F> {
        self.parent
    }
}


impl<'a, F> ForkData<'a, F> where F: Read + Seek {
    fn new(parent: &'a FileSystem<F>, offset: u64) -> ForkData<'a, F> {
        ForkData {
            parent: parent,
            offset: offset,
        }
    }

    pub fn get_logical_size(&self) -> fs::Result<u64> {
        self.read_number(0)
    }

    pub fn get_clump_size(&self) -> fs::Result<u32> {
        self.read_number(8)
    }

    pub fn get_total_blocks(&self) -> fs::Result<u32> {
        self.read_number(12)
    }

    pub fn num_extent_descriptors(&self) -> usize {
        8
    }

    pub fn get_extent_descriptor(&self, index: usize) -> ExtentDescriptor<'a, F> {
        assert!(index < self.num_extent_descriptors());
        ExtentDescriptor::new(self.parent,
                              self.offset + OFFSET_FORK_DATA_EXTENT_RECORD + SIZE_EXTENT_RECORD * index as u64)
    }
}

impl<'a, F> Display for ForkData<'a, F> where F: Read + Seek {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        writeln!(fmt, "Logical size: {:?}", self.get_logical_size())?;
        writeln!(fmt, "Clump size: {:?}", self.get_clump_size())?;
        writeln!(fmt, "Total blocks: {:?}", self.get_total_blocks())?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct ExtentDescriptor<'a, F> where F: 'a {
    parent: &'a FileSystem<F>,
    offset: u64,
}

impl<'a, F> Structure<F> for ExtentDescriptor<'a, F> where F: 'a {
    fn get_offset(&self) -> u64 {
        self.offset
    }

    fn get_filesystem(&self) -> &FileSystem<F> {
        self.parent
    }
}

impl<'a, F> ExtentDescriptor<'a, F> where F: Read + Seek {
    fn new(parent: &'a FileSystem<F>, offset: u64) -> ExtentDescriptor<'a, F> {
        ExtentDescriptor {
            parent: parent,
            offset: offset,
        }
    }

    pub fn get_start_block(&self) -> fs::Result<u32> {
        self.read_number(0)
    }

    pub fn get_block_count(&self) -> fs::Result<u32> {
        self.read_number(4)
    }
}

impl<'a, F> Display for ExtentDescriptor<'a, F> where F: Read + Seek {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        writeln!(fmt, "Start block: {:?}", self.get_start_block())?;
        writeln!(fmt, "Block count: {:?}", self.get_block_count())?;
        Ok(())
    }
}
