use std::mem;
use std::mem::transmute;
use std::ptr;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use allocators::{Allocator, DefaultHeap};

pub trait Chunker {
    fn chunk_size(&self) -> usize;
    fn name(&self) -> &str;
    fn with_chunk_size(&self, size: usize) -> Box<Chunker>;
    fn with_name(&self, name: &str) -> Box<Chunker>;
    fn child(&self, suffix: &str) -> Box<Chunker>;
    fn create_chunk(&mut self) -> *mut u8;
    fn load_chunk(&mut self, _index: usize) -> *mut u8 {
        self.create_chunk()
    }
    fn destroy_chunk(&mut self, ptr: *mut u8);
}

#[derive(Clone)]
pub struct MemChunker {
    name: String,
    chunk_size: usize
}

impl MemChunker {
    pub fn new(name: &str, chunk_size: usize) -> Box<Chunker> {
        Box::new(MemChunker {
            name: String::from(name),
            chunk_size: chunk_size
        })
    }
}

impl Chunker for MemChunker {
    fn chunk_size(&self) -> usize {self.chunk_size}
    fn name(&self) -> &str {self.name.as_str()}

    fn with_chunk_size(&self, size: usize) -> Box<Chunker> {
        let mut new = self.clone();
        new.chunk_size = size;
        Box::new(new)
    }

    fn with_name(&self, name: &str) -> Box<Chunker> {
        let mut new = self.clone();
        new.name = String::from(name);
        Box::new(new)
    }

    fn child(&self, suffix: &str) -> Box<Chunker> {
        Box::new(MemChunker {
            name: self.name.clone() + suffix,
            chunk_size: self.chunk_size
        })
    }

    fn create_chunk(&mut self) -> *mut u8 {
        DefaultHeap::allocate::<u8>(self.chunk_size)
    }

    fn destroy_chunk(&mut self, ptr: *mut u8) {
        unsafe {
            DefaultHeap::deallocate(ptr, self.chunk_size);
        }
    }
}

pub struct ValueInChunk<T> {
    chunker: Box<Chunker>,
    ptr: *mut u8,
    _marker: PhantomData<T>
}

impl<T> ValueInChunk<T> {
    fn new(chunker: Box<Chunker>, default: T) -> ValueInChunk<T> {
        let mut chunker = chunker.with_chunk_size(mem::size_of::<T>());
        let ptr = chunker.create_chunk();
        unsafe {
            *(ptr as *mut T) = default;
        }
        ValueInChunk{
            chunker: chunker,
            ptr: ptr,
            _marker: PhantomData
        }
    }
}

impl<T> Deref for ValueInChunk<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe {
            (self.ptr as *const T).as_ref().unwrap()
        }
    }
}

impl<T> DerefMut for ValueInChunk<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe {
            (self.ptr as *mut T).as_mut().unwrap()
        }
    }
}

pub trait SizedChunkedCollection {
    fn new(chunker: Box<Chunker>, item_size: usize) -> Self;
}

pub struct SizedChunkedArena {
    chunker: Box<Chunker>,
    chunks: Vec<*mut u8>,
    item_size: usize,
    len: ValueInChunk<usize>
}

impl SizedChunkedArena {
    fn items_per_chunk(&self) -> usize {
        self.chunker.chunk_size() / self.item_size
    }

    fn pop_chunk(&mut self) {
        let ptr = self.chunks.pop().unwrap();
        self.chunker.destroy_chunk(ptr);
    }

    pub fn len(&self) -> usize {
        *self.len
    }

    pub fn push(&mut self) -> (*mut u8, usize) {
        if (*self.len + 1) > self.chunks.len() * self.items_per_chunk() {
            self.chunks.push(self.chunker.create_chunk());
        }
        let offset = (*self.len % self.items_per_chunk()) * self.item_size;
        let index = *self.len;
        *self.len += 1;
        unsafe {
            (self.chunks.last_mut().unwrap().offset(offset as isize), index)
        }
    }

    pub fn pop_away(&mut self) {
        *self.len -= 1;
        if *self.len + self.items_per_chunk() < self.chunks.len() * self.items_per_chunk() {
            self.pop_chunk();
        }
    }

    pub unsafe fn swap_remove(&mut self, index: usize) -> Option<*const u8> {
        let last_index = *self.len - 1;
        if last_index == index {
            self.pop_away();
            None
        } else {
            let last = self.at(*self.len - 1);
            let at_index = self.at_mut(index);
            ptr::copy_nonoverlapping(last, at_index, self.item_size);
            self.pop_away();
            Some(self.at(index))
        }
    }

    pub unsafe fn at(&self, index: usize) -> *const u8 {
        self.chunks[index / self.items_per_chunk()]
            .offset(((index % self.items_per_chunk()) * self.item_size) as isize)
    }

    pub unsafe fn at_mut(&mut self, index: usize) -> *mut u8 {
        let items_per_chunk = self.items_per_chunk();
        self.chunks[index / items_per_chunk]
            .offset(((index % items_per_chunk) * self.item_size) as isize)
    }
}

impl SizedChunkedCollection for SizedChunkedArena {
    fn new(chunker: Box<Chunker>, item_size: usize) -> Self {
        assert!(chunker.chunk_size() >= item_size);
        let mut arena = SizedChunkedArena {
            len: ValueInChunk::new(chunker.child("_len"), 0),
            chunker: chunker,
            chunks: Vec::new(),
            item_size: item_size,
        };

        while arena.chunks.len() < *arena.len / arena.items_per_chunk() {
            let next_chunk_index = arena.chunks.len();
            arena.chunks.push(arena.chunker.load_chunk(next_chunk_index));
        }

        arena
    }
}

pub struct ChunkedVec<Item: Clone> {
    arena: SizedChunkedArena,
    marker: PhantomData<Item>
}

impl <Item: Clone> ChunkedVec<Item> {
    pub fn new(chunker: Box<Chunker>) -> Self {
        ChunkedVec {
            arena: SizedChunkedArena::new(chunker, mem::size_of::<Item>()),
            marker: PhantomData
        }
    }

    pub fn len(&self) -> usize {
        *self.arena.len
    }

    pub fn at(&self, index: usize) -> &Item {
        unsafe {
            let item_ptr : &Item = transmute(self.arena.at(index));
            return item_ptr;
        }
    }

    pub fn at_mut(&mut self, index: usize) -> &mut Item {
        unsafe {
            let item_ptr : &mut Item = transmute(self.arena.at(index));
            return item_ptr;
        }
    }

    pub fn push(&mut self, item: Item) {
        unsafe {
            let item_ptr = self.arena.push().0 as *mut Item;
            *item_ptr = item;
        }
    }

    pub fn pop(&mut self) -> Option<Item> {
        if *self.arena.len == 0 {
            None
        } else {
            unsafe {
                let item_ptr : *const Item = transmute(self.arena.at(*self.arena.len - 1));
                self.arena.pop_away();
                Some((*item_ptr).clone())
            }
        }
    }
}

// TODO: replace this by concurrent MPSC queue
// add write_done and read_done indices
// if one thread finishes writing out-of-order,
// let it busy wait for the slower thread then
// increase the write_done counter
pub struct SizedChunkedQueue {
    chunker: Box<Chunker>,
    chunks: Vec<*mut u8>,
    item_size: usize,
    start_index: ValueInChunk<usize>,
    pub read_index: ValueInChunk<usize>,
    pub write_index: ValueInChunk<usize>,
    end_index: ValueInChunk<usize>,
    n_dropped_chunks: ValueInChunk<usize>
}

impl SizedChunkedQueue {
    fn items_per_chunk(&self) -> usize {
        self.chunker.chunk_size() / self.item_size
    }

    // TODO: separate into enqueue_start and enqueue_done
    // or return done_guard
    pub unsafe fn enqueue(&mut self) -> *mut u8 {
        if *self.write_index >= *self.end_index {
            self.chunks.push(self.chunker.create_chunk());
            *self.end_index += self.items_per_chunk();
        }

        let offset = ((*self.write_index % self.items_per_chunk()) * self.item_size) as isize;
        let ptr = self.chunks.last_mut().unwrap().offset(offset);
        *self.write_index += 1;
        ptr
    }

    // TODO: separate into dequeue_start and dequeue_done
    // or return done_guard
    pub unsafe fn dequeue(&mut self) -> Option<*const u8> {
        if *self.read_index == *self.write_index {
            None
        } else {
            let offset = ((*self.read_index % self.items_per_chunk()) * self.item_size) as isize;
            let ptr = self.chunks[0].offset(offset);
            *self.read_index += 1;
            if *self.read_index >= (*self.n_dropped_chunks + 1) * self.items_per_chunk() {
                self.chunks.remove(0); // TODO: remove file?
                *self.n_dropped_chunks += 1;
            }
            Some(ptr)
        }
    }
}

impl SizedChunkedCollection for SizedChunkedQueue {
    fn new(chunker: Box<Chunker>, item_size: usize) -> Self {
        assert!(chunker.chunk_size() >= item_size);

        let mut queue = SizedChunkedQueue {
            start_index: ValueInChunk::new(chunker.child("_start"), 0),
            read_index: ValueInChunk::new(chunker.child("_read"), 0),
            write_index: ValueInChunk::new(chunker.child("_write"), 0),
            end_index: ValueInChunk::new(chunker.child("_end"), 0),
            n_dropped_chunks: ValueInChunk::new(chunker.child("_end"), 0),
            chunker: chunker,
            chunks: Vec::new(),
            item_size: item_size,
        };

        let mut chunk_index = *queue.start_index;
        while chunk_index < *queue.end_index {
            queue.chunks.push(queue.chunker.load_chunk(chunk_index));
            chunk_index += queue.chunker.chunk_size();
        }

        queue
    }
}

pub struct MultiSized<B: SizedChunkedCollection> {
    chunker: Box<Chunker>,
    base_size: usize,
    pub collections: Vec<B>,
    largest_size: ValueInChunk<usize>
}

impl<B: SizedChunkedCollection> MultiSized<B> {
    pub fn new(chunker: Box<Chunker>, base_size: usize) -> Self {
        let mut multi_sized = MultiSized{
            largest_size: ValueInChunk::new(chunker.child("_largest"), 0),
            chunker: chunker,
            collections: Vec::new(),
            base_size: base_size
        };

        while multi_sized.collections.len() < *multi_sized.largest_size {
            multi_sized.push_higher_sized_collection();
        }

        multi_sized
    }

    fn push_higher_sized_collection(&mut self) {
        let new_largest_size = 2u32.pow(self.collections.len() as u32) as usize * self.base_size;
        self.collections.push(B::new(
            self.chunker.child(format!("_{}", new_largest_size).as_str()),
            new_largest_size,
        ))
    }

    pub fn size_to_index(&self, size: usize) -> usize {
        // TODO: the log two part can probably optimized crazily: http://stackoverflow.com/a/11398748
        //       |----------- rounding up int div -----------|
        return (((size + self.base_size - 1) / self.base_size).next_power_of_two() as f32).log2() as usize;
    }

    pub fn sized_for_mut(&mut self, size: usize) -> &mut B {
        let index = self.size_to_index(size);

        while *self.largest_size <= index {
            self.push_higher_sized_collection();
            *self.largest_size += 1;
        }

        &mut self.collections[index]
    }
}