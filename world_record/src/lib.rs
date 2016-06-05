#![allow(dead_code)]
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

type IDRepr = u32;

pub struct ID<T> {
   id: IDRepr,
   marker: PhantomData<T>
}

impl<T> Copy for ID<T> {}
impl<T> Clone for ID<T> {
    fn clone(&self) -> ID<T> {*self}
}

#[derive(Clone, Copy)]
pub struct Record<T> {
    id: ID<T>,
    rc: T
}

impl<T> Deref for Record<T> {
    type Target = T;
    
    fn deref(&self) -> &T {
        return &self.rc;
    }
}

impl<T> DerefMut for Record<T> {
    fn deref_mut(&mut self) -> &mut T {
        return &mut self.rc;
    }
}

pub trait FutureState {
    fn materialize(&mut self);
}

mod growable_buffer;
mod growable_vec;
mod slot_map;
mod record_collection;
mod future_record_collection;

pub use growable_buffer::GrowableBuffer;
pub use record_collection::RecordCollection;
pub use future_record_collection::FutureRecordCollection;