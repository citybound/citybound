//! Tools for dealing with large amounts of identical actors
use compact::Compact;
use super::chunked::{MemChunker, ValueInChunk, SizedChunkedArena, MultiSized};
use super::slot_map::{SlotIndices, SlotMap};
use super::messaging::{Message, Packet, Fate};
use super::actor_system::{World, Actor};
use super::id::{TypedID, RawID, broadcast_instance_id};
use std::marker::PhantomData;

/// A container-like actor, housing many instances of identical behaviour.
///
/// Offers efficient storage of and broadcasting to its instances.
///
/// New instances can be added to a swarm using [`Create`](struct.Create.html)
/// or [`CreateWith`](struct.CreateWith.html).
pub struct Swarm<Actor> {
    instances: MultiSized<SizedChunkedArena>,
    slot_map: SlotMap,
    n_instances: ValueInChunk<usize>,
    _marker: PhantomData<[Actor]>,
}

const CHUNK_SIZE: usize = 4096 * 4096 * 16;

impl<A: Actor + Clone> Swarm<A> {
    /// Create an empty `Swarm`.
    #[allow(new_without_default)]
    pub fn new() -> Self {
        let chunker = MemChunker::from_settings("", CHUNK_SIZE);
        Swarm {
            instances: MultiSized::new(chunker.child("_instances"), A::typical_size()),
            n_instances: ValueInChunk::new(chunker.child("_n_instances"), 0),
            slot_map: SlotMap::new(chunker.child("_slot_map")),
            _marker: PhantomData,
        }
    }

    fn allocate_instance_id(&mut self) -> (usize, usize) {
        self.slot_map.allocate_id()
    }

    fn at_index_mut(&mut self, index: SlotIndices) -> &mut A {
        unsafe { &mut *(self.instances.bins[index.bin()].at_mut(index.slot()) as *mut A) }
    }

    fn at_mut(&mut self, id: usize, version: u8) -> Option<&mut A> {
        self.slot_map.indices_of(id, version).map(move |index| {
            self.at_index_mut(index)
        })
    }

    /// Allocate a instance RawID for later use when manually adding a instance (see `add_with_id`)
    pub unsafe fn allocate_id(&mut self, base_id: RawID) -> RawID {
        let (instance_id, version) = self.allocate_instance_id();
        RawID::new(
            base_id.type_id,
            instance_id as u32,
            base_id.machine,
            version as u8,
        )
    }

    /// used externally when manually adding a instance,
    /// making use of a previously allocated RawID (see `allocate_id`)
    pub unsafe fn add_manually_with_id(&mut self, initial_state: *mut A, id: RawID) {
        self.add_with_id(initial_state, id);
        *self.n_instances += 1;
    }

    /// Used internally
    unsafe fn add_with_id(&mut self, initial_state: *mut A, id: RawID) {
        let size = (*initial_state).total_size_bytes();
        let bin_index = self.instances.size_to_index(size);
        let bin = &mut self.instances.bin_for_size_mut(size);
        let (ptr, index) = bin.push();

        self.slot_map.associate(
            id.instance_id as usize,
            SlotIndices::new(bin_index, index),
        );
        assert_eq!(
            self.slot_map
                .indices_of_no_version_check(id.instance_id as usize)
                .bin(),
            bin_index
        );

        Compact::compact_behind(initial_state, ptr as *mut A);
        let actor_in_slot = &mut *(ptr as *mut A);
        actor_in_slot.set_id(id);
    }

    fn swap_remove(&mut self, indices: SlotIndices) -> bool {
        unsafe {
            let bin = &mut self.instances.bins[indices.bin()];
            match bin.swap_remove(indices.slot()) {
                Some(ptr) => {
                    let swapped_actor = &*(ptr as *mut A);
                    self.slot_map.associate(
                        swapped_actor.id().as_raw().instance_id as
                            usize,
                        indices,
                    );
                    true
                }
                None => false,
            }

        }
    }

    fn remove(&mut self, id: RawID) {
        let i = self.slot_map.indices_of_no_version_check(
            id.instance_id as usize,
        );
        self.remove_at_index(i, id);
    }

    fn remove_at_index(&mut self, i: SlotIndices, id: RawID) {
        // TODO: not sure if this is the best place to drop actor state
        let old_actor_ptr = self.at_index_mut(i) as *mut A;
        unsafe {
            ::std::ptr::drop_in_place(old_actor_ptr);
        }
        self.swap_remove(i);
        self.slot_map.free(
            id.instance_id as usize,
            id.version as usize,
        );
        *self.n_instances -= 1;
    }

    fn resize(&mut self, id: usize) -> bool {
        let index = self.slot_map.indices_of_no_version_check(id);
        self.resize_at_index(index)
    }

    fn resize_at_index(&mut self, old_i: SlotIndices) -> bool {
        let old_actor_ptr = self.at_index_mut(old_i) as *mut A;
        unsafe { self.add_with_id(old_actor_ptr, (*old_actor_ptr).id().as_raw()) };
        self.swap_remove(old_i)
    }

    fn receive_instance<M: Message, H>(
        &mut self,
        packet: &Packet<M>,
        handler: &H,
        world: &mut World,
    ) where
        H: Fn(&M, &mut A, &mut World) -> Fate + 'static,
    {
        let (fate, is_still_compact) = {
            if let Some(actor) = self.at_mut(
                packet.recipient_id.instance_id as usize,
                packet.recipient_id.version,
            )
            {
                let fate = handler(&packet.message, actor, world);
                (fate, actor.is_still_compact())
            } else {
                println!(
                    "Tried to send {} packet to {} actor of wrong version!",
                    unsafe {::std::intrinsics::type_name::<M>()},
                    unsafe {::std::intrinsics::type_name::<A>()}
                );
                return;
            }
        };

        match fate {
            Fate::Live => {
                if !is_still_compact {
                    self.resize(packet.recipient_id.instance_id as usize);
                }
            }
            Fate::Die => self.remove(packet.recipient_id),
        }
    }

    fn receive_broadcast<M: Message, H>(
        &mut self,
        packet: &Packet<M>,
        handler: &H,
        world: &mut World,
    ) where
        H: Fn(&M, &mut A, &mut World) -> Fate + 'static,
    {
        // this function has to deal with the fact that during the iteration,
        // receivers of the broadcast can be resized
        // and thus removed from a bin, swapping in either
        //    - other receivers that didn't receive the broadcast yet
        //    - resized and added receivers that alredy received the broadcast
        //    - sub actors that were created during one of the broadcast receive handlers,
        //      that shouldn't receive this broadcast
        // the only assumption is that no sub actors are immediately completely deleted

        let recipients_todo_per_bin: Vec<usize> = {
            self.instances.bins.iter().map(|bin| bin.len()).collect()
        };

        let n_bins = self.instances.bins.len();

        for (c, recipients_todo) in recipients_todo_per_bin.iter().enumerate().take(n_bins) {
            let mut slot = 0;
            let mut index_after_last_recipient = *recipients_todo;

            for _ in 0..*recipients_todo {
                let index = SlotIndices::new(c, slot);
                let (fate, is_still_compact, id) = {
                    let actor = self.at_index_mut(index);
                    let fate = handler(&packet.message, actor, world);
                    (fate, actor.is_still_compact(), actor.id().as_raw())
                };

                let repeat_slot = match fate {
                    Fate::Live => {
                        if is_still_compact {
                            false
                        } else {
                            self.resize_at_index(index);
                            // this should also work in the case where the "resized" actor
                            // itself is added to the same bin again
                            let swapped_in_another_receiver = self.instances.bins[c].len() <
                                index_after_last_recipient;
                            if swapped_in_another_receiver {
                                index_after_last_recipient -= 1;
                                true
                            } else {
                                false
                            }
                        }
                    }
                    Fate::Die => {
                        self.remove_at_index(index, id);
                        // this should also work in the case where the "resized" actor
                        // itself is added to the same bin again
                        let swapped_in_another_receiver = self.instances.bins[c].len() <
                            index_after_last_recipient;
                        if swapped_in_another_receiver {
                            index_after_last_recipient -= 1;
                            true
                        } else {
                            false
                        }
                    }
                };

                if !repeat_slot {
                    slot += 1;
                }
            }
        }
    }

    pub fn dispatch_packet<M: Message, F>(
        &mut self,
        packet: &Packet<M>,
        handler: &F,
        world: &mut World,
    ) where
        F: Fn(&M, &mut A, &mut World) -> Fate + 'static,
    {
        if packet.recipient_id.instance_id == broadcast_instance_id() {
            self.receive_broadcast(packet, handler, world);
        } else {
            self.receive_instance(packet, handler, world);
        }
    }
}

use super::actor_system::InstancesCountable;
impl<A: Actor> InstancesCountable for Swarm<A> {
    fn instance_count(&self) -> usize {
        *self.n_instances
    }
}
