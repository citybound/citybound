//! This crate offers an abstraction over allocating fixed-size chunks of memory
//! and different low-level collection types making use of these chunks to emulate
//! "infinite" dynamically growing storage.
//!
//! Its purpose is being able to abstract storage of entity-collections
//! (such as actors in `Kay`) over both temporary heap memory and persistent
//! mmap'ed memory used for both runtime and savegames.

#![warn(missing_docs)]
#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]
#![feature(box_syntax)]

use std::mem;
use std::mem::transmute;
use std::ptr;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

extern crate allocators;
use allocators::{Allocator, DefaultHeap};

/// Describes a strategy for creation and destruction of chunks, adhering to configurable settings
pub trait Chunker {
    /// Get the set chunk size
    fn chunk_size(&self) -> usize;
    /// Get the set chunk name root
    ///
    /// This name can be used internally to store a chunk in a specific file.
    fn name(&self) -> &str;
    /// Create an identical chunker, but with different chunk size
    fn with_chunk_size(&self, size: usize) -> Box<Chunker>;
    /// Create an identical chunker, but with a different name
    fn with_name(&self, name: &str) -> Box<Chunker>;
    /// Create an identical chunker, with the name of the current chunker extended by a suffix.
    ///
    /// This can be used to store subproperties of complex objects in similar-sounding files
    fn child(&self, suffix: &str) -> Box<Chunker>;
    /// Create a new chunk with the set chunk size
    fn create_chunk(&mut self) -> *mut u8;
    /// Load a persisted chunk that was previously created by this chunker,
    /// given an index to identify the particular chunk.
    /// *Note:* the default implementation just creates a new, empty chunk
    // TODO: Actually load the chunks
    // TODO: Report back if chunk existed or not
    fn load_chunk(&mut self, _index: usize) -> *mut u8 {
        self.create_chunk()
    }
    /// Destroys a chunk that was created by this chunker
    /// as well as any persisted representation of the chunk.
    ///
    /// Undefined behaviour if passed a string that was not
    /// created by `create_chunk` or `load_chunk`
    unsafe fn destroy_chunk(&mut self, ptr: *mut u8);
}

/// Implementation of `Chunker` for temporary heap memory
#[derive(Clone)]
pub struct MemChunker {
    name: String,
    chunk_size: usize,
}

impl MemChunker {
    /// Create a new `MemChunker` with the given settings
    ///
    /// (This is not in the `Chunker` trait, because this receiver-less fn would
    /// prevent it from being an `Object`, see `error[E0038]`)
    pub fn from_settings(name: &str, chunk_size: usize) -> Box<Chunker> {
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

    unsafe fn destroy_chunk(&mut self, ptr: *mut u8) {
        // TODO: remove file?
        DefaultHeap::deallocate(ptr, self.chunk_size);
    }
}

/// A simple value, stored in a chunk. Typically used to store metadata of collections
/// (such as length) using the same kind of (non-)persistence as the items of the collection
pub struct ValueInChunk<T> {
    ptr: *mut u8,
    chunker: Box<Chunker>,
    _marker: PhantomData<T>,
}

impl<T> ValueInChunk<T> {
    /// Create a new value with a given default.
    /// The default is used if no persisted chunk for this value was found.
    pub fn new(chunker: Box<Chunker>, default: T) -> ValueInChunk<T> {
        let mut chunker = chunker.with_chunk_size(mem::size_of::<T>());
        // TODO: try to load an existing chunk
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
        unsafe {
            ::std::ptr::drop_in_place(self.ptr);
            self.chunker.destroy_chunk(self.ptr);
        };
    }
}

/// Any kind of dynamically-growing collection of fixed-size items that uses chunks
///
/// Exists so `MultiSized` can be built generically using
/// any kind of fixed-size collection (see `MultiSized`)
pub trait SizedChunkedCollection {
    /// Create a new collection based on a chunker and item size
    fn new(chunker: Box<Chunker>, item_size: usize) -> Self;
}

/// A simple array-like collection of fixed-size items of unknown type
pub struct SizedChunkedArena {
    /// Chunker which is used to create and destroy chunks as needed
    pub chunker: Box<Chunker>,
    /// List of allocated chunks
    pub chunks: Vec<*mut u8>,
    /// Item size in bytes
    pub item_size: usize,
    len: ValueInChunk<usize>,
}

impl SizedChunkedArena {
    fn items_per_chunk(&self) -> usize {
        self.chunker.chunk_size() / self.item_size
    }

    fn pop_chunk(&mut self) {
        let ptr = self.chunks.pop().unwrap();
        unsafe {
            self.chunker.destroy_chunk(ptr);
        }
    }

    /// Number of elements in the collection
    pub fn len(&self) -> usize {
        *self.len
    }

    /// Is the collection empty?
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Allocate space for a new item, returns a pointer to where the new item
    /// can be written to and the index that the new item will have.
    ///
    /// This is handled like this so items of heterogeneous types or sizes less
    /// than the fixed item size can be added to the collection.
    pub fn push(&mut self) -> (*mut u8, usize) {
        // Make sure the item can fit in the current chunk
        if (*self.len + 1) > self.chunks.len() * self.items_per_chunk() {
            // If not, create a new chunk
            self.chunks.push(self.chunker.create_chunk());
        }
        let offset = (*self.len % self.items_per_chunk()) * self.item_size;
        let index = *self.len;
        *self.len += 1;
        unsafe {
            (
                self.chunks.last_mut().unwrap().offset(offset as isize),
                index,
            )
        }
    }

    /// Remove the last item from the end
    pub fn pop_away(&mut self) {
        *self.len -= 1;
        // If possible, remove the last chunk as well
        if *self.len + self.items_per_chunk() < self.chunks.len() * self.items_per_chunk() {
            self.pop_chunk();
        }
    }

    /// Remove the item at index, by swapping it with the last item
    /// and then popping, returning the removed item, if it existed.
    ///
    /// This is a O(1) way of removing an item if the order of items doesn't matter.
    pub unsafe fn swap_remove(&mut self, index: usize) -> Option<*const u8> {
        assert!(*self.len > 0);
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
        self.chunks[index / self.items_per_chunk()].offset(
            ((index % self.items_per_chunk()) * self.item_size) as
                isize,
        )
    }

    /// Get a mutable pointer to the item at `index`
    pub unsafe fn at_mut(&mut self, index: usize) -> *mut u8 {
        let items_per_chunk = self.items_per_chunk();
        self.chunks[index / items_per_chunk].offset(
            ((index % items_per_chunk) * self.item_size) as
                isize,
        )
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
            arena.chunks.push(
                arena.chunker.load_chunk(next_chunk_index),
            );
        }

        arena
    }
}

/// A vector which stores items of a known type in a `SizedChunkedArena`
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

    /// Get the number of elements in the vector
    pub fn len(&self) -> usize {
        *self.arena.len
    }

    /// Is the chunked vector empty?
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get a reference to the item at `index`
    pub fn at(&self, index: usize) -> &Item {
        unsafe { &*(self.arena.at(index) as *const Item) }
    }

    /// Get a mutable reference to the item at `index`
    pub fn at_mut(&mut self, index: usize) -> &mut Item {
        unsafe { &mut *(self.arena.at(index) as *mut Item) }
    }

    /// Push an item onto the vector
    pub fn push(&mut self, item: Item) {
        unsafe {
            let item_ptr = self.arena.push().0 as *mut Item;
            *item_ptr = item;
        }
    }

    /// Remove and return the last item, if the vector wasn't empty
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

/// A FIFO queue which stores heterogeneously sized items
// TODO: replace this by concurrent MPSC queue
//       add write_done and read_done indices
//       if one thread finishes writing out-of-order,
//       let it busy wait for the slower thread then
//       increase the write_done counter
pub struct ChunkedQueue {
    chunker: Box<Chunker>,
    chunks: Vec<*mut u8>,
    start_offset: ValueInChunk<usize>,
    read_offset: ValueInChunk<usize>,
    write_offset: ValueInChunk<usize>,
    end_offset: ValueInChunk<usize>,
    len: ValueInChunk<usize>,
    chunks_to_drop: Vec<*mut u8>,
}

// TODO invent a container struct with NonZero instead
const JUMP_TO_NEXT_CHUNK: usize = 0;

impl ChunkedQueue {
    /// Create a new chunked queue based on a chunker
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

        // if the persisted end_offset is > 0, persisted chunks need to be loaded
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

    /// Number of items in the queue
    pub fn len(&self) -> usize {
        *self.len
    }

    /// Is the queue empty?
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Enqueue an item of a given size. Returns a pointer that the item can be written to.
    ///
    /// This is handled like this so items of heterogeneous types can be enqueued.
    // TODO: return done_guard to mark as concurrently readable
    pub unsafe fn enqueue(&mut self, size: usize) -> *mut u8 {
        let total_size = size + mem::size_of::<usize>();
        let offset = *self.write_offset % self.chunker.chunk_size();
        let ptr = self.chunks.last_mut().unwrap().offset(offset as isize);
        // one more size marker needs to fit, even if it will just be a jump marker!
        if *self.write_offset + total_size + mem::size_of::<usize>() < *self.end_offset {
            // store the item size as a header
            *(ptr as *mut usize) = total_size;
            let payload_ptr = ptr.offset(mem::size_of::<usize>() as isize);
            *self.write_offset += total_size;
            *self.len += 1;
            // return the pointer to where the item can be written
            payload_ptr
        // the item won't fit in the current chunk anymore
        } else {
            // store a jump marker instead of item size
            *(ptr as *mut usize) = JUMP_TO_NEXT_CHUNK;
            self.chunks.push(self.chunker.create_chunk());
            // retry at the beginning of a new chunk
            *self.write_offset = *self.end_offset;
            *self.end_offset += self.chunker.chunk_size();
            self.enqueue(size)
        }
    }

    /// Dequeue an item. Returns a pointer to the item in the queue, unless the queue is empty.
    // TODO: return done_guard to mark as droppable
    pub unsafe fn dequeue(&mut self) -> Option<*const u8> {
        // Queue is empty
        if *self.read_offset == *self.write_offset {
            None
        } else {
            let offset = *self.read_offset % self.chunker.chunk_size();
            let ptr = self.chunks[0].offset(offset as isize);
            let total_size = *(ptr as *mut usize);
            // instead of an item size, a jump marker was stored
            if total_size == JUMP_TO_NEXT_CHUNK {
                // retry at the beginning of the next chunk
                *self.read_offset += self.chunker.chunk_size() - offset;
                self.chunks_to_drop.push(self.chunks.remove(0));
                self.dequeue()
            } else {
                let payload_ptr = ptr.offset(mem::size_of::<usize>() as isize);
                *self.read_offset += total_size;
                *self.len -= 1;
                // return pointer to where the item is stored
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

/// Based on a collection type for fixed-size items ("Bin"), creates a collection for
/// heterogenously-sized items which will be stored in the most appropriately-sized bin.
///
/// All Bins will use children of a main chunker to create their chunks.
pub struct MultiSized<Bin: SizedChunkedCollection> {
    chunker: Box<Chunker>,
    base_size: usize,
    /// All fixed-size bins in this multi-sized collection
    ///
    /// The bin at index `i` will have item-size `base_size * 2 ^ i`
    pub bins: Vec<Bin>,
    largest_size: ValueInChunk<usize>,
}

impl<B: SizedChunkedCollection> MultiSized<B> {
    /// Create a new `MultiSized` collection using the given chunker as a main chunker
    /// and a base size that represents the smallest expected item size
    /// (this will be used as the item size of the smallest-sized Bin)
    pub fn new(chunker: Box<Chunker>, base_size: usize) -> Self {
        let mut multi_sized = MultiSized {
            largest_size: ValueInChunk::new(chunker.child("_largest"), 0),
            chunker: chunker,
            bins: Vec::new(),
            base_size: base_size,
        };

        while multi_sized.bins.len() < *multi_sized.largest_size {
            multi_sized.push_larger_sized_bin();
        }

        multi_sized
    }

    /// Add a new Bin which has double the size of the previously largest one
    fn push_larger_sized_bin(&mut self) {
        let new_largest_size = 2u32.pow(self.bins.len() as u32) as usize * self.base_size;
        self.bins.push(B::new(
            self.chunker.child(
                format!("_{}", new_largest_size).as_str(),
            ),
            new_largest_size,
        ))
    }

    /// Get the index of the Bin which stores items of size `size`
    pub fn size_to_index(&self, size: usize) -> usize {
        (((size + self.base_size - 1) / self.base_size).next_power_of_two() as f32).log2() as usize
    }

    /// Get a reference to the Bin most appropriately sized given item size `size`
    pub fn bin_for_size_mut(&mut self, size: usize) -> &mut B {
        let index = self.size_to_index(size);

        while *self.largest_size <= index {
            self.push_larger_sized_bin();
            *self.largest_size += 1;
        }

        &mut self.bins[index]
    }
}
