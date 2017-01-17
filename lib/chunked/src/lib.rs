#![feature(plugin)]
#![plugin(clippy)]
#![feature(box_syntax)]

use ::std::mem;
use ::std::mem::transmute;
use ::std::ptr;
use ::std::marker::PhantomData;
use ::std::ops::{Deref, DerefMut};

extern crate allocators;
use allocators::{Allocator, DefaultHeap};

/// Store information and utility functions for the creation of chunks
pub trait Chunker {
    fn chunk_size(&self) -> usize;
    fn name(&self) -> &str;
    /// Create new chunker with different size
    fn with_chunk_size(&self, size: usize) -> Box<Chunker>;
    /// Create new chunker with different name
    fn with_name(&self, name: &str) -> Box<Chunker>;
    /// Create new chunker with a suffix
    fn child(&self, suffix: &str) -> Box<Chunker>;
    /// Allocate memory
    fn create_chunk(&mut self) -> *mut u8;
    /// Load chunks from disk/persistent storage
    // TODO: Actually load the chunks
    fn load_chunk(&mut self, _index: usize) -> *mut u8 {
        self.create_chunk()
    }
    /// Deallocate chunk
    fn destroy_chunk(&mut self, ptr: *mut u8);
}

/// Implementation of `Chunker` for non-volatile memory
#[derive(Clone)]
pub struct MemChunker {
    name: String,
    chunk_size: usize,
}

impl MemChunker {
    pub fn new(name: &str, chunk_size: usize) -> Box<Chunker> {
        box MemChunker {
            name: String::from(name),
            chunk_size: chunk_size,
        }
    }
}

impl Chunker for MemChunker {
    fn chunk_size(&self) -> usize {
        self.chunk_size
    }
    fn name(&self) -> &str {
        self.name.as_str()
    }

    fn with_chunk_size(&self, size: usize) -> Box<Chunker> {
        let mut new = self.clone();
        new.chunk_size = size;
        box new
    }

    fn with_name(&self, name: &str) -> Box<Chunker> {
        let mut new = self.clone();
        new.name = String::from(name);
        box new
    }

    fn child(&self, suffix: &str) -> Box<Chunker> {
        box MemChunker {
            name: self.name.clone() + suffix,
            chunk_size: self.chunk_size,
        }
    }

    fn create_chunk(&mut self) -> *mut u8 {
        DefaultHeap::allocate::<u8>(self.chunk_size)
    }

    fn destroy_chunk(&mut self, ptr: *mut u8) {
        // TODO: remove file?
        unsafe {
            DefaultHeap::deallocate(ptr, self.chunk_size);
        }
    }
}

/// An value that is directly stored within a newly created chunk which can fit exactly 1 of the
/// type
pub struct ValueInChunk<T> {
    ptr: *mut u8,
    chunker: Box<Chunker>,
    _marker: PhantomData<T>,
}

impl<T> ValueInChunk<T> {
    /// Create a new chunk and allocating enough memory to fit one of `T`, copying the default
    /// value across to it
    pub fn new(chunker: Box<Chunker>, default: T) -> ValueInChunk<T> {
        let mut chunker = chunker.with_chunk_size(mem::size_of::<T>());
        let ptr = chunker.create_chunk();
        unsafe {
            *(ptr as *mut T) = default;
        }
        ValueInChunk {
            ptr: ptr,
            chunker: chunker,
            _marker: PhantomData,
        }
    }
}

impl<T> Deref for ValueInChunk<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { (self.ptr as *const T).as_ref().unwrap() }
    }
}

impl<T> DerefMut for ValueInChunk<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { (self.ptr as *mut T).as_mut().unwrap() }
    }
}

impl<T> Drop for ValueInChunk<T> {
    fn drop(&mut self) {
        unsafe { ::std::ptr::drop_in_place(self.ptr) };
        self.chunker.destroy_chunk(self.ptr);
    }
}

pub trait SizedChunkedCollection {
    /// Create a collection of fixed-size items using a chunker
    fn new(chunker: Box<Chunker>, item_size: usize) -> Self;
}

/// Provides storage of items up to a fixed size in chunks
pub struct SizedChunkedArena {
    /// Chunker which is used for the creation of all chunks
    pub chunker: Box<Chunker>,
    /// List of allocated chunks
    pub chunks: Vec<*mut u8>,
    /// Size, in bytes, of each element
    pub item_size: usize,
    /// Length, in elements, stored as a value in a chunk
    /// For ease of saving and loading
    len: ValueInChunk<usize>,
}

impl SizedChunkedArena {
    /// Calculates the amount of items that can fit in a single chunk, rounding down
    fn items_per_chunk(&self) -> usize {
        self.chunker.chunk_size() / self.item_size
    }

    /// Removes the last chunk from the end
    fn pop_chunk(&mut self) {
        let ptr = self.chunks.pop().unwrap();
        self.chunker.destroy_chunk(ptr);
    }

    /// Get the amount of items (not chunks) stored
    pub fn len(&self) -> usize {
        *self.len
    }

    /// Add an new item to the end
    pub fn push(&mut self) -> (*mut u8, usize) {
        // Make sure the item can fit in the current chunk
        if (*self.len + 1) > self.chunks.len() * self.items_per_chunk() {
            // If not, create a new chunk
            self.chunks.push(self.chunker.create_chunk());
        }
        let offset = (*self.len % self.items_per_chunk()) * self.item_size;
        let index = *self.len;
        *self.len += 1;
        unsafe { (self.chunks.last_mut().unwrap().offset(offset as isize), index) }
    }

    /// Remove the last item from the end
    pub fn pop_away(&mut self) {
        *self.len -= 1;
        // If possible, remove the last chunk as well
        if *self.len + self.items_per_chunk() < self.chunks.len() * self.items_per_chunk() {
            self.pop_chunk();
        }
    }

    /// Swap the item at `index` with the item at the end, and then pop the item at the end,
    /// possibly returning the item at the end
    pub unsafe fn swap_remove(&mut self, index: usize) -> Option<*const u8> {
        let last_index = *self.len - 1;
        if last_index == index {
            // if swapping last item
            self.pop_away();
            None
        } else {
            let last = self.at(*self.len - 1);
            let at_index = self.at_mut(index);
            // copy item from index to the end
            ptr::copy_nonoverlapping(last, at_index, self.item_size);
            self.pop_away();
            Some(self.at(index))
        }
    }

    /// Get a pointer to the item at `index`
    pub unsafe fn at(&self, index: usize) -> *const u8 {
        self.chunks[index / self.items_per_chunk()]
            .offset(((index % self.items_per_chunk()) * self.item_size) as isize)
    }

    /// Get a mutable pointer to the item at `index`
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

/// A vector which stores the data in chunks
pub struct ChunkedVec<Item: Clone> {
    arena: SizedChunkedArena,
    marker: PhantomData<Item>,
}

impl<Item: Clone> ChunkedVec<Item> {
    /// Create a new chunked vector
    pub fn new(chunker: Box<Chunker>) -> Self {
        ChunkedVec {
            arena: SizedChunkedArena::new(chunker, mem::size_of::<Item>()),
            marker: PhantomData,
        }
    }

    /// Get the length of the vector
    pub fn len(&self) -> usize {
        *self.arena.len
    }

    /// Get the item at the index
    pub fn at(&self, index: usize) -> &Item {
        unsafe { &*(self.arena.at(index) as *const Item) }
    }

    /// Get the item at the index mutably
    pub fn at_mut(&mut self, index: usize) -> &mut Item {
        unsafe { &mut *(self.arena.at(index) as *mut Item) }
    }

    /// Push a item to the back
    pub fn push(&mut self, item: Item) {
        unsafe {
            let item_ptr = self.arena.push().0 as *mut Item;
            *item_ptr = item;
        }
    }

    /// Remove and return the last item
    pub fn pop(&mut self) -> Option<Item> {
        if *self.arena.len == 0 {
            None
        } else {
            unsafe {
                let item_ptr: *const Item = transmute(self.arena.at(*self.arena.len - 1));
                self.arena.pop_away();
                Some((*item_ptr).clone())
            }
        }
    }
}

/// A FIFO queue which stores a heterogeneously sized items, implemented in chunks
// TODO: replace this by concurrent MPSC queue
//       add write_done and read_done indices
//       if one thread finishes writing out-of-order,
//       let it busy wait for the slower thread then
//       increase the write_done counter
pub struct ChunkedQueue {
    chunker: Box<Chunker>,
    chunks: Vec<*mut u8>,
    start_offset: ValueInChunk<usize>,
    pub read_offset: ValueInChunk<usize>,
    pub write_offset: ValueInChunk<usize>,
    end_offset: ValueInChunk<usize>,
    len: ValueInChunk<usize>,
    chunks_to_drop: Vec<*mut u8>,
}

// TODO invent a container struct with NonZero instead
const JUMP_TO_NEXT_CHUNK: usize = 0;

impl ChunkedQueue {
    pub fn new(chunker: Box<Chunker>) -> Self {
        let mut queue = ChunkedQueue {
            start_offset: ValueInChunk::new(chunker.child("_start"), 0),
            read_offset: ValueInChunk::new(chunker.child("_read"), 0),
            write_offset: ValueInChunk::new(chunker.child("_write"), 0),
            end_offset: ValueInChunk::new(chunker.child("_end"), 0),
            len: ValueInChunk::new(chunker.child("_len"), 0),
            chunker: chunker,
            chunks: Vec::new(),
            chunks_to_drop: Vec::new(),
        };

        let mut chunk_index = *queue.start_offset;
        while chunk_index < *queue.end_offset {
            queue.chunks.push(queue.chunker.load_chunk(chunk_index));
            chunk_index += queue.chunker.chunk_size();
        }

        if queue.chunks.is_empty() {
            queue.chunks.push(queue.chunker.create_chunk());
            *queue.end_offset = queue.chunker.chunk_size();
        }

        queue
    }

    pub fn len(&self) -> usize {
        *self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Gets a pointer to the back of the queue
    // TODO: return done_guard to mark as concurrently readable
    pub unsafe fn enqueue(&mut self, size: usize) -> *mut u8 {
        let total_size = size + mem::size_of::<usize>();
        let offset = *self.write_offset % self.chunker.chunk_size();
        let ptr = self.chunks.last_mut().unwrap().offset(offset as isize);
        if *self.write_offset + total_size >= *self.end_offset {
            *(ptr as *mut usize) = JUMP_TO_NEXT_CHUNK;
            self.chunks.push(self.chunker.create_chunk());
            *self.write_offset = *self.end_offset;
            *self.end_offset += self.chunker.chunk_size();
            self.enqueue(size)
        } else {
            *(ptr as *mut usize) = total_size;
            let payload_ptr = ptr.offset(mem::size_of::<usize>() as isize);
            *self.write_offset += total_size;
            *self.len += 1;
            payload_ptr
        }
    }

    /// Gets a pointer to the front of the queue, queuing chunks at the front for deletion
    // TODO: return done_guard to mark as droppable
    pub unsafe fn dequeue(&mut self) -> Option<*const u8> {
        if *self.read_offset == *self.write_offset {
            None
        } else {
            let offset = *self.read_offset % self.chunker.chunk_size();
            let ptr = self.chunks[0].offset(offset as isize);
            let total_size = *(ptr as *mut usize);
            if total_size == JUMP_TO_NEXT_CHUNK {
                *self.read_offset += self.chunker.chunk_size() - offset;
                self.chunks_to_drop.push(self.chunks.remove(0));
                self.dequeue()
            } else {
                let payload_ptr = ptr.offset(mem::size_of::<usize>() as isize);
                *self.read_offset += total_size;
                *self.len -= 1;
                Some(payload_ptr)
            }
        }
    }

    /// Delete chunks which have already been read
    pub unsafe fn drop_old_chunks(&mut self) {
        for chunk in self.chunks_to_drop.drain(..) {
            self.chunker.destroy_chunk(chunk);
        }
    }
}

/// Storage for objects with dynamic sizes
pub struct MultiSized<B: SizedChunkedCollection> {
    chunker: Box<Chunker>,
    base_size: usize,
    pub collections: Vec<B>,
    largest_size: ValueInChunk<usize>,
}

impl<B: SizedChunkedCollection> MultiSized<B> {
    /// Create a new `MultiSized` collection
    pub fn new(chunker: Box<Chunker>, base_size: usize) -> Self {
        let mut multi_sized = MultiSized {
            largest_size: ValueInChunk::new(chunker.child("_largest"), 0),
            chunker: chunker,
            collections: Vec::new(),
            base_size: base_size,
        };

        while multi_sized.collections.len() < *multi_sized.largest_size {
            multi_sized.push_higher_sized_collection();
        }

        multi_sized
    }

    /// Create a new chunked storage which has a 2 times size of the previous one
    fn push_higher_sized_collection(&mut self) {
        let new_largest_size = 2u32.pow(self.collections.len() as u32) as usize * self.base_size;
        self.collections.push(B::new(self.chunker.child(format!("_{}", new_largest_size).as_str()),
                                     new_largest_size))
    }

    /// Get the index of the chunked storage which stores the size of the object
    pub fn size_to_index(&self, size: usize) -> usize {
        // TODO: the log two part can probably optimized crazily:
        // http://stackoverflow.com/a/11398748
        // ----------- rounding up int div -----------|
        (((size + self.base_size - 1) / self.base_size).next_power_of_two() as f32).log2() as usize
    }

    /// Get a pointer to the suitable sized chunked storage
    pub fn sized_for_mut(&mut self, size: usize) -> &mut B {
        let index = self.size_to_index(size);

        while *self.largest_size <= index {
            self.push_higher_sized_collection();
            *self.largest_size += 1;
        }

        &mut self.collections[index]
    }
}
