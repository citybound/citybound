use super::compact::{Compact};
use super::chunked::{Chunker, SizedChunkedArena, MultiSized};
use super::slot_map::{SlotIndices, SlotMap};
use super::messaging::{Actor, Recipient, Message};
use super::actor_system::{LivingActor, ID, World};
use ::std::marker::PhantomData;
use ::std::mem::transmute;

pub struct Swarm<Actor> {
    actors: MultiSized<SizedChunkedArena>,
    slot_map: SlotMap,
    _marker: PhantomData<[Actor]>
}

impl<A: Actor> Swarm<A> {
    pub fn new(chunker: Box<Chunker>, base_size: usize) -> Self {
        Swarm{
            actors: MultiSized::new(chunker.child("_actors"), base_size),
            slot_map: SlotMap::new(chunker.child("_slot_map")),
            _marker: PhantomData
        }
    }

    pub fn allocate_instance_id(&mut self) -> u32 {
        self.slot_map.allocate_id() as u32
    }

    fn at_index(&self, index: SlotIndices) -> &LivingActor<A> {
        unsafe {&*(self.actors.collections[index.collection()].at(index.slot()) as *const LivingActor<A>)}
    }

    fn at_index_mut(&mut self, index: SlotIndices) -> &mut LivingActor<A> {
        unsafe {&mut *(self.actors.collections[index.collection()].at_mut(index.slot()) as *mut LivingActor<A>)}
    }

    pub fn at(&self, id: usize) -> &LivingActor<A> {
        self.at_index(*self.slot_map.indices_of(id))
    }

    pub fn at_mut(&mut self, id: usize) -> &mut LivingActor<A> {
        let index = *self.slot_map.indices_of(id);
        self.at_index_mut(index)
    }

    pub fn add(&mut self, actor: &LivingActor<A>) {
        let size = actor.total_size_bytes();
        let collection_index = self.actors.size_to_index(size);
        let ref mut collection = self.actors.sized_for_mut(size);
        let (ptr, index) = collection.push();

        self.slot_map.associate(actor.id.instance_id as usize, SlotIndices::new(collection_index, index));
        assert!(self.slot_map.indices_of(actor.id.instance_id as usize).collection()== collection_index);

        unsafe {
            let actor_in_slot : &mut LivingActor<A> = transmute(ptr);
            actor_in_slot.compact_behind_from(&actor);
        }
    }

    fn swap_remove(&mut self, indices: SlotIndices) -> bool {
        unsafe {
            let ref mut collection = self.actors.collections[indices.collection()];
            match collection.swap_remove(indices.slot()) {
                Some(ptr) => {
                    let swapped_actor : &LivingActor<A> = transmute(ptr);
                    self.slot_map.associate(swapped_actor.id.instance_id as usize, indices);
                    true
                },
                None => false
            }
            
        }
    }

    pub fn remove(&mut self, id: usize) {
        let i = *self.slot_map.indices_of(id);
        self.swap_remove(i);
        self.slot_map.free(id);
    }

    pub fn resize(&mut self, id: usize) -> bool {
        let index = *self.slot_map.indices_of(id);
        self.resize_at_index(index)
    }

    fn resize_at_index(&mut self, old_i: SlotIndices) -> bool {
        let old_actor_ptr = self.at_index(old_i) as *const LivingActor<A>;
        self.add(unsafe{&*old_actor_ptr});
        self.swap_remove(old_i)
    }

    pub fn receive_instance<M: Message>(&mut self, message: &M, system: &mut World, id: ID) where A: Recipient<M> {
        let is_still_compact = {
            let actor = self.at_mut(id.instance_id as usize);
            actor.receive(message, system, id);
            actor.is_still_compact()
        };

        if !is_still_compact {
            self.resize(id.instance_id as usize);
        }
    }

    pub fn receive_broadcast<M: Message>(&mut self, message: &M, system: &mut World) where A: Recipient<M> {
        // this function has to deal with the fact that during the iteration, receivers of the broadcast can be resized
        // and thus removed from a collection, swapping in either
        //    - other receivers that didn't receive the broadcast yet
        //    - resized and added receivers that alredy received the broadcast
        //    - actors that were created during one of the broadcast receive handlers, that shouldn't receive this broadcast
        // the only assumption is that no actors are immediately completely deleted

        let receivers_todo_per_collection : Vec<usize> = {
            self.actors.collections.iter().map(|collection| {collection.len()}).collect()
        };

        let n_collections = self.actors.collections.len();

        for c in 0..n_collections {
            let mut slot = 0;
            let receivers_todo = receivers_todo_per_collection[c];
            let mut index_after_last_receiver = receivers_todo;

            for _ in 0..receivers_todo {
                let index = SlotIndices::new(c, slot);
                let is_still_compact = {
                    let actor = self.at_index_mut(index);
                    let actor_id = actor.id;
                    actor.receive(message, system, actor_id);
                    actor.is_still_compact()
                };

                let repeat_slot = if is_still_compact {
                    false
                } else {
                    self.resize_at_index(index);
                    // this should also work in the case where the "resized" actor itself is added to the same collection again
                    let swapped_in_another_receiver = self.actors.collections[c].len() < index_after_last_receiver;
                    if swapped_in_another_receiver {
                        index_after_last_receiver -= 1;
                        true
                    } else {
                        false
                    }
                };

                if !repeat_slot {
                    slot += 1;
                }
            }
        }
    }
}

impl <M: Message, A: Actor + Recipient<M>> Recipient<M> for Swarm<A> {
    fn receive(&mut self, message: &M, world: &mut World, recipient_id: ID) {
        if recipient_id.is_broadcast() {
            self.receive_broadcast(message, world);
        } else {
            self.receive_instance(message, world, recipient_id);
        }
    }
}