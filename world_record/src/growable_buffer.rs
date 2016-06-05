extern crate memmap;
extern crate std;

use self::memmap::{Mmap, Protection};
use std::mem;
use std::ptr;
use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

pub struct MmapValue<T> {
    ptr: *mut T
}

impl<T> std::ops::Deref for MmapValue<T> {
    type Target = T;
    
    fn deref (&self) -> &T {
        unsafe {
            return &*(self.ptr);
        }
    }
}

impl<T> std::ops::DerefMut for MmapValue<T> {    
    fn deref_mut (&mut self) -> &mut T {
        unsafe {
            return &mut *(self.ptr);
        }
    }
}

pub struct MmapSlice<T> {
    ptr: *mut T,
    len: usize
}

impl<T> std::ops::Deref for MmapSlice<T> {
    type Target = [T];
    
    fn deref (&self) -> &[T] {
        unsafe {
            return std::slice::from_raw_parts(self.ptr, self.len);
        }
    }
}

impl<T> std::ops::DerefMut for MmapSlice<T> {    
    fn deref_mut (&mut self) -> &mut [T] {
        unsafe {
            return std::slice::from_raw_parts_mut(self.ptr, self.len);
        }
    }
}

pub struct GrowableBuffer<Header, Item> {
    path: PathBuf,
    mmap: Mmap,
    pub header: MmapValue<Header>,
    pub items: MmapSlice<Item>,
}

fn open_or_create_file_with_min_size(path: &Path, min_size: usize) -> std::io::Result<(File, bool)> {
    let mut file = try!(OpenOptions::new().create(true).read(true).write(true).open(path));
    let metadata = try!(file.metadata());
    let initial_length = metadata.len();
    if initial_length < min_size as u64 {
        try!(file.seek(SeekFrom::Start((min_size - 1) as u64)));
        try!(file.write(&[0]));
        try!(file.flush());
    }
    let is_new_file = initial_length == 0;
    return Ok((file, is_new_file));
}

impl<Header: Default, Item> GrowableBuffer<Header, Item> {
    pub fn new(path: PathBuf) -> GrowableBuffer<Header, Item> {
        let real_path = path.with_extension("bin");
        std::fs::create_dir_all(&real_path.parent().unwrap()).unwrap();
        let min_size = Self::header_in_items() * mem::size_of::<Item>();
        let (file, is_new_file) = open_or_create_file_with_min_size(&real_path, min_size).unwrap();
        // TODO: create default header if new file!
        let mut mmap = Mmap::open(&file, Protection::ReadWrite).unwrap();
        unsafe {
            return GrowableBuffer {
                path: real_path,
                header: Self::header_from_mmap(&mut mmap),
                items: Self::items_from_mmap(&mut mmap),
                mmap: mmap,
            };
        }
    }
    
    unsafe fn header_from_mmap(mmap: &mut Mmap) -> MmapValue<Header> {
        return MmapValue{
            ptr: mem::transmute(mmap.mut_ptr())
        };
    }
    
    unsafe fn items_from_mmap(mmap: &mut Mmap) -> MmapSlice<Item> {
        let base_ptr : *mut Item = mem::transmute(mmap.mut_ptr());
        let ptr = base_ptr.offset(Self::header_in_items() as isize);
        let n_items = (mmap.len() / mem::size_of::<Item>()) - Self::header_in_items();
        return MmapSlice{
            ptr: ptr,
            len: n_items
        }
    }

    fn header_in_items() -> usize {
        let div = mem::size_of::<Header>() / mem::size_of::<Item>();
        let rest = mem::size_of::<Header>() % mem::size_of::<Item>();
        return if rest == 0 {
            div
        } else {
            div + 1
        };
    }

    pub fn require_cap(&mut self, required_cap: usize) {
        if required_cap > self.item_cap() || required_cap < self.item_cap() / 2 {
            let new_size_bytes = (Self::header_in_items() + required_cap).next_power_of_two() *
                                mem::size_of::<Item>();
            self.mmap.flush().unwrap();
            let (file, _) = open_or_create_file_with_min_size(&self.path, new_size_bytes).unwrap();
            let mut new_mmap = Mmap::open(&file, Protection::ReadWrite).unwrap();
            unsafe {
                self.header = Self::header_from_mmap(&mut new_mmap);
                self.items = Self::items_from_mmap(&mut new_mmap);
            }
            self.mmap = new_mmap;
        }
    }

    fn item_cap(&self) -> usize {
        return (self.mmap.len() / mem::size_of::<Item>()) - Self::header_in_items();
    }
    
    pub fn overwrite_with(&mut self, other: &Self) {
        self.require_cap(other.item_cap());
        unsafe {
            ptr::copy_nonoverlapping(other.header.ptr, self.header.ptr, 1);
            ptr::copy_nonoverlapping(other.items.ptr, self.items.ptr, other.item_cap());
        }
    }
}