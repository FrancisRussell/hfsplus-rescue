use chrono::{self, TimeZone};
use error::HFSPError;
use fs;
use num;
use std::fmt::{self, Display, Formatter};
use std::io::{self, Read, Seek, SeekFrom};
use std::mem;
use std::slice;
use std::cmp;
use std::sync::Mutex;

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

    fn read(&self, offset: u64, buff: &mut [u8]) -> io::Result<usize> where F: Read + Seek {
        let mut file = self.get_filesystem().file.lock().unwrap();
        file.seek(SeekFrom::Start(self.get_offset() + offset))?;
        file.read(buff)
    }

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

    fn read_date(&self, offset: usize, is_local: bool) -> fs::Result<chrono::DateTime<chrono::Local>> where F: Read + Seek {
        let seconds: u32 = self.read_number(offset)?;
        let duration = chrono::Duration::seconds(seconds as i64);
        let origin_date = chrono::NaiveDate::from_ymd(1904, 1, 1);
        let origin_time = chrono::NaiveTime::from_hms(0,0,0);
        let origin = chrono::NaiveDateTime::new(origin_date, origin_time);

        let date = if is_local {
            chrono::Local.from_local_datetime(&origin).single().unwrap() + duration
        } else {
            chrono::Local.from_utc_datetime(&origin) + duration
        };
        Ok(date)
    }
}

impl<F> Structure<F> for FileSystem<F> {
    fn get_offset(&self) -> u64 {
        0
    }

    fn get_filesystem(&self) -> &FileSystem<F> {
        self
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

    pub fn get_modify_date(&self) -> fs::Result<chrono::DateTime<chrono::Local>> {
        self.read_date(20, false)
    }

    pub fn get_backup_date(&self) -> fs::Result<chrono::DateTime<chrono::Local>> {
        self.read_date(24, false)
    }

    pub fn get_checked_date(&self) -> fs::Result<chrono::DateTime<chrono::Local>> {
        self.read_date(24, false)
    }

    pub fn get_fork_data_allocation(&self) -> ForkData<'a, F> {
        ForkData::new(self.parent, self.offset + OFFSET_VOLUME_HEADER_FORKS)
    }

    pub fn get_file_allocation(&self) -> fs::Result<HFSFile<'a, F>> {
        HFSFile::new(self.parent, self.get_fork_data_allocation())
    }

    pub fn get_fork_data_extents(&self) -> ForkData<'a, F> {
        ForkData::new(self.parent, self.offset + OFFSET_VOLUME_HEADER_FORKS + SIZE_FORK_DATA)
    }

    pub fn get_file_extents(&self) -> fs::Result<HFSFile<'a, F>> {
        HFSFile::new(self.parent, self.get_fork_data_extents())
    }

    pub fn get_fork_data_catalog(&self) -> ForkData<'a, F> {
        ForkData::new(self.parent, self.offset + OFFSET_VOLUME_HEADER_FORKS + SIZE_FORK_DATA * 2)
    }

    pub fn get_file_catalog(&self) -> fs::Result<HFSFile<'a, F>> {
        HFSFile::new(self.parent, self.get_fork_data_catalog())
    }

    pub fn get_fork_data_attributes(&self) -> ForkData<'a, F> {
        ForkData::new(self.parent, self.offset + OFFSET_VOLUME_HEADER_FORKS + SIZE_FORK_DATA * 3)
    }

    pub fn get_file_attributes(&self) -> fs::Result<HFSFile<'a, F>> {
        HFSFile::new(self.parent, self.get_fork_data_attributes())
    }

    pub fn get_fork_data_startup(&self) -> ForkData<'a, F> {
        ForkData::new(self.parent, self.offset + OFFSET_VOLUME_HEADER_FORKS + SIZE_FORK_DATA * 4)
    }

    pub fn get_file_startup(&self) -> fs::Result<HFSFile<'a, F>> {
        HFSFile::new(self.parent, self.get_fork_data_startup())
    }
}

impl<'a, F> Display for VolumeHeader<'a, F> where F: Read + Seek {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        writeln!(fmt, "Version: {:?}", self.get_version())?;
        writeln!(fmt, "Folder count: {:?}", self.get_folder_count())?;
        writeln!(fmt, "Modify date: {:?}", self.get_modify_date())?;
        writeln!(fmt, "Backup date: {:?}", self.get_backup_date())?;
        writeln!(fmt, "Checked date: {:?}", self.get_checked_date())?;
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

#[derive(Debug)]
pub struct HFSFile<'a, F> where F: 'a {
    parent: &'a FileSystem<F>,
    length: u64,
    block_size: u64,
    offsets: Vec<(u64, u32)>,
    offset: u64,
}

impl<'a, F> HFSFile<'a, F> where F: Read + Seek {
    // TODO: Extent overflow support
    // TODO: Read truncated files when later extents are damaged
    fn new(parent: &'a FileSystem<F>, fork_data: ForkData<'a, F>) -> fs::Result<HFSFile<'a, F>> {
        let length = fork_data.get_logical_size()?;
        let block_size = parent.get_volume_header()?.get_block_size()?;

        let mut offsets = Vec::new();
        let mut seen_blocks = 0;
        for idx in 0..(fork_data.num_extent_descriptors()) {
            let end_offset_bytes = seen_blocks as u64 * block_size as u64;
            if end_offset_bytes >= length {
                break;
            }
            let descriptor = fork_data.get_extent_descriptor(idx);
            offsets.push((end_offset_bytes, descriptor.get_start_block()?));
            seen_blocks += descriptor.get_block_count()?;
        }

        let end_offset_bytes = seen_blocks as u64 * block_size as u64;
        if end_offset_bytes < length {
            return Err(HFSPError::ExtentOverflowNotSupported);
        }

        let result = HFSFile {
            parent: parent,
            block_size: block_size as u64,
            length: length,
            offsets: offsets,
            offset: 0,
        };
        Ok(result)
    }
}

impl<'a, F> Read for HFSFile<'a, F> where F: Read + Seek {
    fn read(&mut self, buf: &mut[u8]) -> io::Result<usize> {
        if self.offset > self.length {
            panic!("Cannot read beyond end of file");
        }
        let read_size = cmp::min(buf.len() as u64, self.length - self.offset) as usize;
        if read_size == 0 {
            return Ok(0);
        }
        assert!(!self.offsets.is_empty());
        let extent_index = match self.offsets.binary_search_by_key(&self.offset, |&(o, _)| o) {
            Ok(idx) => idx,
            Err(idx) => idx - 1,
        };
        let extent_offset = self.offsets[extent_index].1 as u64 * self.block_size;
        let intra_extent_offset = self.offset - self.offsets[extent_index].0;
        let fs_offset = extent_offset + intra_extent_offset;
        let read = self.parent.read(fs_offset, &mut buf[0..read_size])?;
        self.offset += read as u64;
        Ok(read)
    }
}

impl<'a, F> Seek for HFSFile<'a, F> where F: Read + Seek {
    // TODO: Handle invalid seeks better
    fn seek(&mut self, from: io::SeekFrom) -> io::Result<u64> {
        match from {
            io::SeekFrom::Start(offset) => {
                self.offset = offset;
            },
            io::SeekFrom::End(offset) => {
                if offset >= 0 {
                    self.offset = self.length + (offset as u64);
                } else {
                    let offset_magnitude = (-offset) as u64;
                    if offset_magnitude > self.length {
                        panic!("Cannot seek before start of file");
                    } else {
                        self.offset = self.length - offset_magnitude;
                    }
                }
            },
            io::SeekFrom::Current(offset) => {
                if offset >= 0 {
                    let offset = offset as u64;
                    self.offset += offset;
                } else {
                    let offset_magnitude = (-offset) as u64;
                    if offset_magnitude > self.offset {
                        panic!("Cannot seek before start of file");
                    } else {
                        self.offset -= offset_magnitude;
                    }
                }
            },
        }
        Ok(self.offset)
    }
}
