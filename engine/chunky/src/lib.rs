//! This crate offers an abstraction over allocating fixed-size chunks of memory
//! and different low-level collection types making use of these chunks to emulate
//! "infinite" dynamically growing storages for heterogeneously-sized items.
//!
//! Its purpose is being able to abstract storage of entity-collections
//! (such as actors in `Kay`) over both temporary heap memory and persistent
//! mmap'ed memory used for both runtime and savegames.

#![warn(missing_docs)]
#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]
#![feature(vec_resize_default)]
#![feature(conservative_impl_trait)]

use std::mem;
use std::mem::transmute;
use std::ptr;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

extern crate allocators;
use allocators::{Allocator, DefaultHeap};

/// Identifies a chunk or chunk group uniquely - to be used for persistence
#[derive(Clone)]
pub struct Ident(pub String);

impl Ident {
    /// Create a sub-identifier within a group
    pub fn sub<T: ::std::fmt::Display>(&self, suffix: T) -> Ident {
        Ident(format!("{}:{}", self.0, suffix))
    }
}

impl<T: ::std::fmt::Display> From<T> for Ident {
    fn from(source: T) -> Self {
        Ident(format!("{}", source))
    }
}

struct Chunk<H: Handler> {
    ptr: *mut u8,
    size: usize,
    _h: PhantomData<*const H>,
}

impl<H: Handler> Chunk<H> {
    /// load a chunk with a given identifier, or create it if it didn't exist
    pub fn load_or_create(ident: Ident, size: usize) -> (Chunk<H>, bool) {
        let (ptr, created_new) = H::load_or_create_chunk(ident, size);
        (Chunk { ptr, size, _h: PhantomData }, created_new)
    }

    /// load a chunk with a given identifier, assumes it exists
    pub fn load(ident: Ident) -> Chunk<H> {
        let (ptr, size) = H::load_chunk(ident);
        Chunk { ptr, size, _h: PhantomData }
    }

    /// load a chunk with a given identifier, assumes it doesn't exist
    pub fn create(ident: Ident, size: usize) -> Chunk<H> {
        let ptr = H::create_chunk(ident, size);
        Chunk { ptr, size, _h: PhantomData }
    }

    /// destroy a chunk and delete any persisted representation
    pub fn forget_forever(self) {
        unsafe {
            H::destroy_chunk(self.ptr, self.size);
        }
        mem::forget(self);
    }
}

impl<H: Handler> Drop for Chunk<H> {
    fn drop(&mut self) {
        unsafe { H::unload_chunk(self.ptr, self.size) }
    }
}

/// A strategy for managing chunks
pub trait Handler: Sized {
    /// Create a new chunk with a given identifier, assumes it doesn't exist
    fn create_chunk(ident: Ident, size: usize) -> *mut u8;
    /// Load a chunk with a given identifier, or create it if it doesn't exist
    fn load_or_create_chunk(ident: Ident, size: usize) -> (*mut u8, bool);
    /// Load a chunk with a given identifier, assumes it exists
    fn load_chunk(ident: Ident) -> (*mut u8, usize);
    /// Deallocate a chunk, but keep any persisted representation of it
    unsafe fn unload_chunk(ptr: *mut u8, size: usize);
    /// Deallocate a chunk and delete any persisted representation of it
    unsafe fn destroy_chunk(ptr: *mut u8, size: usize);
}

/// A `Handler` that allocates chunks on the heap
pub struct HeapHandler;

impl Handler for HeapHandler {
    fn create_chunk(_ident: Ident, size: usize) -> *mut u8 {
        //println!("Allocating chunk {} of size {}", ident.0, size);
        DefaultHeap::allocate(size)
    }

    fn load_or_create_chunk(ident: Ident, size: usize) -> (*mut u8, bool) {
        (Self::create_chunk(ident, size), true)
    }

    fn load_chunk(_ident: Ident) -> (*mut u8, usize) {
        panic!("can't load memory based chunks");
    }

    unsafe fn unload_chunk(ptr: *mut u8, size: usize) {
        DefaultHeap::deallocate(ptr, size);
    }

    unsafe fn destroy_chunk(ptr: *mut u8, size: usize) {
        Self::unload_chunk(ptr, size);
    }
}

/// A single value stored in a chunk
pub struct Value<V, H: Handler> {
    chunk: Chunk<H>,
    _marker: PhantomData<*mut V>,
}

impl<V, H: Handler> Value<V, H> {
    /// Load the value in the chunk with the given identifier, or create it using a default value
    pub fn load_or_default(ident: Ident, default: V) -> Value<V, H> {
        let (chunk, created_new) = Chunk::load_or_create(ident, mem::size_of::<V>());

        if created_new {
            unsafe {
                ptr::write(chunk.ptr as *mut V, default);
            }
        }

        Value { chunk, _marker: PhantomData }
    }
}

impl<V, H: Handler> Deref for Value<V, H> {
    type Target = V;

    fn deref(&self) -> &V {
        unsafe { (self.chunk.ptr as *const V).as_ref().unwrap() }
    }
}

impl<V, H: Handler> DerefMut for Value<V, H> {
    fn deref_mut(&mut self) -> &mut V {
        unsafe { (self.chunk.ptr as *mut V).as_mut().unwrap() }
    }
}

impl<V, H: Handler> Drop for Value<V, H> {
    fn drop(&mut self) {
        unsafe {
            ::std::ptr::drop_in_place(self.chunk.ptr);
        };
    }
}

/// Refers to an item within an `Arena`
#[derive(Copy, Clone)]
pub struct ArenaIndex(pub usize);

/// Stores items of a fixed (max) size consecutively in a collection of chunks
pub struct Arena<H: Handler> {
    ident: Ident,
    chunks: Vec<Chunk<H>>,
    chunk_size: usize,
    item_size: usize,
    len: Value<usize, H>,
}

impl<H: Handler> Arena<H> {
    /// Create a new arena given a chunk group identifier, chunk size and (max) item size
    pub fn new(ident: Ident, chunk_size: usize, item_size: usize) -> Arena<H> {
        assert!(chunk_size >= item_size);

        let len = Value::<usize, H>::load_or_default(ident.sub("len"), 0);
        let mut chunks = Vec::new();

        for i in 0..*len {
            chunks.push(Chunk::<H>::load(ident.sub(i)));
        }

        Arena {
            ident,
            chunks,
            chunk_size,
            item_size,
            len,
        }
    }

    fn items_per_chunk(&self) -> usize {
        self.chunk_size / self.item_size
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
    pub fn push(&mut self) -> (*mut u8, ArenaIndex) {
        // Make sure the item can fit in the current chunk
        if (*self.len + 1) > self.chunks.len() * self.items_per_chunk() {
            // If not, create a new chunk
            self.chunks.push(Chunk::create(
                self.ident.sub(*self.len),
                self.chunk_size,
            ));
        }
        let offset = (*self.len % self.items_per_chunk()) * self.item_size;
        let index = ArenaIndex(*self.len);
        *self.len += 1;
        unsafe {
            (
                self.chunks.last_mut().unwrap().ptr.offset(offset as isize),
                index,
            )
        }
    }

    /// Remove the last item from the end
    pub fn pop_away(&mut self) {
        *self.len -= 1;
        // If possible, remove the last chunk as well
        if *self.len + self.items_per_chunk() < self.chunks.len() * self.items_per_chunk() {
            self.chunks
                .pop()
                .expect("should have chunk left")
                .forget_forever();
        }
    }

    /// Remove the item at index, by swapping it with the last item
    /// and then popping, returning the swapped in item (unless empty).
    ///
    /// This is a O(1) way of removing an item if the order of items doesn't matter.
    pub unsafe fn swap_remove(&mut self, index: ArenaIndex) -> Option<*const u8> {
        assert!(*self.len > 0);
        let last_index = *self.len - 1;
        if last_index == index.0 {
            // if swapping last item
            self.pop_away();
            None
        } else {
            let last = self.at(ArenaIndex(*self.len - 1));
            let at_index = self.at_mut(index);
            ptr::copy_nonoverlapping(last, at_index, self.item_size);
            self.pop_away();
            Some(self.at(index))
        }
    }

    /// Get a pointer to the item at `index`
    pub unsafe fn at(&self, index: ArenaIndex) -> *const u8 {
        self.chunks[index.0 / self.items_per_chunk()].ptr.offset(
            ((index.0 % self.items_per_chunk()) *
                 self.item_size) as isize,
        )
    }

    /// Get a mutable pointer to the item at `index`
    pub unsafe fn at_mut(&mut self, index: ArenaIndex) -> *mut u8 {
        let items_per_chunk = self.items_per_chunk();
        self.chunks[index.0 / items_per_chunk].ptr.offset(
            ((index.0 % items_per_chunk) * self.item_size) as
                isize,
        )
    }
}

/// A vector which stores items of a known type in an `Arena`
pub struct Vector<Item: Clone, H: Handler> {
    arena: Arena<H>,
    marker: PhantomData<Item>,
}

impl<Item: Clone, H: Handler> Vector<Item, H> {
    /// Create a new chunky vector
    pub fn new(ident: Ident, chunk_size: usize) -> Self {
        let item_size = mem::size_of::<Item>();
        Vector {
            arena: Arena::new(ident, ::std::cmp::max(item_size, chunk_size), item_size),
            marker: PhantomData,
        }
    }

    /// Get the number of elements in the vector
    pub fn len(&self) -> usize {
        *self.arena.len
    }

    /// Is the chunky vector empty?
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get a reference to the item at `index`
    pub fn at(&self, index: usize) -> &Item {
        assert!(index < self.len());
        unsafe { &*(self.arena.at(ArenaIndex(index)) as *const Item) }
    }

    /// Get a mutable reference to the item at `index`
    pub fn at_mut(&mut self, index: usize) -> &mut Item {
        assert!(index < self.len());
        unsafe { &mut *(self.arena.at(ArenaIndex(index)) as *mut Item) }
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
                let item_ptr: *const Item =
                    transmute(self.arena.at(ArenaIndex(*self.arena.len - 1)));
                let item = Some(ptr::read(item_ptr));
                self.arena.pop_away();
                item
            }
        }
    }
}

/// A FIFO queue which stores heterogeneously sized items
pub struct Queue<H: Handler> {
    ident: Ident,
    typical_chunk_size: usize,
    chunks: Vec<Chunk<H>>,
    first_chunk_at: Value<usize, H>,
    last_chunk_at: Value<usize, H>,
    read_at: Value<usize, H>,
    write_at: Value<usize, H>,
    len: Value<usize, H>,
    chunks_to_drop: Vec<Chunk<H>>,
}

// TODO invent a container struct with NonZero instead
enum NextItemRef {
    SameChunk(usize),
    NextChunk,
}

impl<H: Handler> Queue<H> {
    /// Create a new queue
    pub fn new(ident: &Ident, typical_chunk_size: usize) -> Self {
        let mut queue = Queue {
            first_chunk_at: Value::load_or_default(ident.sub("first_chunk"), 0),
            last_chunk_at: Value::load_or_default(ident.sub("last_chunk"), 0),
            read_at: Value::load_or_default(ident.sub("read"), 0),
            write_at: Value::load_or_default(ident.sub("write"), 0),
            len: Value::load_or_default(ident.sub("len"), 0),
            ident: ident.clone(),
            typical_chunk_size,
            chunks: Vec::new(),
            chunks_to_drop: Vec::new(),
        };

        // if the persisted end_offset is > 0, persisted chunks need to be loaded
        if *queue.len > 0 {
            let mut chunk_offset = *queue.first_chunk_at;
            while chunk_offset <= *queue.last_chunk_at {
                let chunk = Chunk::load(ident.sub(chunk_offset));
                chunk_offset += chunk.size;
                queue.chunks.push(chunk);
            }
        }

        if queue.chunks.is_empty() {
            queue.chunks.push(
                Chunk::create(ident.sub(0), typical_chunk_size),
            );
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
        enum EnqueueResult {
            Success(*mut u8),
            RetryInNewChunkOfSize(usize),
        };

        let result = {
            let ref_size = mem::size_of::<NextItemRef>();
            let offset = *self.write_at - *self.last_chunk_at;
            let chunk = self.chunks.last_mut().expect("should always have a chunk");
            let entry_ptr = chunk.ptr.offset(offset as isize);

            // one more next item ref needs to fit afterwards,
            // even if it will just be a jump marker!
            let min_space = ref_size + size + ref_size;

            if offset + min_space <= chunk.size {
                // store the item size as a header
                *(entry_ptr as *mut NextItemRef) = NextItemRef::SameChunk(ref_size + size);
                let payload_ptr = entry_ptr.offset(ref_size as isize);
                *self.write_at += ref_size + size;
                *self.len += 1;
                // return the pointer to where the item can be written
                EnqueueResult::Success(payload_ptr)
            } else {
                //println!("Not enough space. Offset: {}, Min Space: {},
                //          Chunk size: {}", offset, min_space, chunk.size);
                // store a jump marker instead of item size
                *(entry_ptr as *mut NextItemRef) = NextItemRef::NextChunk;
                let new_chunk_size = ::std::cmp::max(self.typical_chunk_size, min_space);
                // retry at the beginning of a new chunk
                *self.last_chunk_at += chunk.size;
                *self.write_at = *self.last_chunk_at;
                EnqueueResult::RetryInNewChunkOfSize(new_chunk_size)
            }
        };

        match result {
            EnqueueResult::Success(payload_ptr) => payload_ptr,
            EnqueueResult::RetryInNewChunkOfSize(new_chunk_size) => {
                self.chunks.push(Chunk::create(
                    self.ident.sub(*self.last_chunk_at),
                    new_chunk_size,
                ));
                self.enqueue(size)
            }
        }
    }

    /// Dequeue an item. Returns a pointer to the item in the queue, unless the queue is empty.
    // TODO: return done_guard to mark as droppable
    pub unsafe fn dequeue(&mut self) -> Option<*const u8> {
        enum DequeueResult {
            Empty,
            Success(*const u8),
            RetryInNextChunk,
        };

        let result = if *self.read_at == *self.write_at {
            DequeueResult::Empty
        } else {
            let offset = *self.read_at - *self.first_chunk_at;
            let chunk = &self.chunks[0];
            let entry_ptr = chunk.ptr.offset(offset as isize);

            match *(entry_ptr as *mut NextItemRef) {
                NextItemRef::NextChunk => {
                    *self.first_chunk_at += chunk.size;
                    *self.read_at = *self.first_chunk_at;
                    DequeueResult::RetryInNextChunk
                }
                NextItemRef::SameChunk(total_size) => {
                    let payload_ptr = entry_ptr.offset(mem::size_of::<NextItemRef>() as isize);
                    *self.read_at += total_size;
                    *self.len -= 1;
                    DequeueResult::Success(payload_ptr)
                }
            }
        };

        match result {
            DequeueResult::Empty => None,
            DequeueResult::Success(payload_ptr) => Some(payload_ptr),
            DequeueResult::RetryInNextChunk => {
                self.chunks_to_drop.push(self.chunks.remove(0));
                self.dequeue()
            }
        }
    }

    /// Delete chunks which have already been read
    pub unsafe fn drop_old_chunks(&mut self) {
        for chunk in self.chunks_to_drop.drain(..) {
            chunk.forget_forever();
        }
    }
}

/// Refers to an item in a `MultiArena`
#[derive(Copy, Clone)]
pub struct MultiArenaIndex(pub usize, pub ArenaIndex);

/// Based on a collection type for fixed-size items ("Bin"), creates a collection for
/// heterogenously-sized items which will be stored in the most appropriately-sized bin.
///
/// All Bins will use children of a main chunker to create their chunks.
pub struct MultiArena<H: Handler> {
    ident: Ident,
    typical_chunk_size: usize,
    base_size: usize,
    /// All fixed-size bins in this multi-sized collection
    ///
    /// The bin at index `i` will have item-size `base_size * 2 ^ i`
    bins: Vec<Option<Arena<H>>>,
    used_bin_sizes: Vector<usize, H>,
}

impl<H: Handler> MultiArena<H> {
    /// Create a new `MultiArena` collection using `Arena` bins and a base size that represents
    /// the smallest expected item size (used as the item size of the smallest-sized bin)
    pub fn new(ident: Ident, typical_chunk_size: usize, base_size: usize) -> Self {
        let mut multi_arena = MultiArena {
            typical_chunk_size,
            base_size,
            used_bin_sizes: Vector::<usize, H>::new(ident.sub("used_bin_sizes"), 1024),
            ident,
            bins: Vec::new(),
        };

        let n_bins = multi_arena.used_bin_sizes.len();

        for i in 0..n_bins {
            let size = *multi_arena.used_bin_sizes.at(i);
            multi_arena.get_or_insert_bin_for_size(size);
        }

        multi_arena
    }

    fn size_rounded_multiple(&self, size: usize) -> usize {
        let size_rounded_to_base_size = (size + self.base_size - 1) / self.base_size;
        size_rounded_to_base_size.next_power_of_two()
    }

    /// Get the index of the Bin which stores items of size `size`
    pub fn size_to_index(&self, size: usize) -> usize {
        (self.size_rounded_multiple(size) as f32).log2() as usize
    }

    fn get_or_insert_bin_for_size(&mut self, size: usize) -> &mut Arena<H> {
        let index = self.size_to_index(size);
        let size_rounded_up = self.size_rounded_multiple(size) * self.base_size;

        if index >= self.bins.len() {
            self.bins.resize_default(index + 1)
        }

        let maybe_bin = &mut self.bins[index];

        if let Some(ref mut bin) = *maybe_bin {
            bin
        } else {
            self.used_bin_sizes.push(size_rounded_up);
            let chunk_size = ::std::cmp::max(self.typical_chunk_size, size_rounded_up);
            *maybe_bin = Some(Arena::new(
                self.ident.sub(size_rounded_up),
                chunk_size,
                size_rounded_up,
            ));
            maybe_bin.as_mut().unwrap()
        }
    }

    /// Get an (untyped) pointer to the item at the given index
    pub fn at(&self, index: MultiArenaIndex) -> *const u8 {
        unsafe {
            self.bins[index.0]
                .as_ref()
                .expect("No bin at this index")
                .at(index.1)
        }
    }

    /// Get an (untyped) mutable pointer to the item at the given index
    pub fn at_mut(&mut self, index: MultiArenaIndex) -> *mut u8 {
        unsafe {
            self.bins[index.0]
                .as_mut()
                .expect("No bin at this index")
                .at_mut(index.1)
        }
    }

    /// Add an item to the end of the bin corresponding to its size
    pub fn push(&mut self, size: usize) -> (*mut u8, MultiArenaIndex) {
        let bin_index = self.size_to_index(size);
        let bin = &mut self.get_or_insert_bin_for_size(size);
        let (ptr, arena_index) = bin.push();
        (ptr, MultiArenaIndex(bin_index, arena_index))
    }

    /// Remove the item referenced by `index` from its bin by swapping with the bin's last item
    pub fn swap_remove_within_bin(&mut self, index: MultiArenaIndex) -> Option<*const u8> {
        unsafe {
            self.bins[index.0]
                .as_mut()
                .expect("No bin at this index")
                .swap_remove(index.1)
        }
    }

    /// Return indices of bins that actually contain items and their respective lengths
    pub fn populated_bin_indices_and_lens<'a>(
        &'a self,
    ) -> impl Iterator<Item = (usize, usize)> + 'a {
        self.bins.iter().enumerate().filter_map(
            |(index, maybe_bin)| {
                maybe_bin.as_ref().map(|bin| (index, bin.len()))
            },
        )
    }

    /// Get the length of the bin of the given bin index
    pub fn bin_len(&self, bin_index: usize) -> usize {
        self.bins[bin_index]
            .as_ref()
            .expect("No bin at this index")
            .len()
    }
}
