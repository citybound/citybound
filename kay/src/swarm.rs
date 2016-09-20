use embedded::{Embedded};
use chunked::{Chunker, SizedChunkedArena, MultiSized};
use slot_map::{SlotIndices, SlotMap};
use messaging::{Recipient, Message};
use std::marker::PhantomData;
use {Actor, ID, ShortTypeId};
use std::mem::transmute;

pub struct Swarm<Actor> {
    actors: MultiSized<SizedChunkedArena>,
    slot_map: SlotMap,
    _marker: PhantomData<[Actor]>
}

impl<S: Embedded> Swarm<Actor<S>>
where Actor<S> : ShortTypeId {
    pub fn new(chunker: Box<Chunker>, base_size: usize) -> Self {
        Swarm{
            actors: MultiSized::new(chunker.child("_actors"), base_size),
            slot_map: SlotMap::new(chunker.child("_slot_map")),
            _marker: PhantomData
        }
    }

    pub fn create(&mut self, state: S) -> Actor<S> {
        Actor {
            id: self.allocate_id(),
            state: state
        }
    }

    pub fn allocate_id(&mut self) -> ID {
        ID {
            type_id: Actor::<S>::type_id() as u16,
            version: 0,
            instance_id: self.slot_map.allocate_id() as u32
        }
    }

    pub fn at(&self, id: usize) -> &Actor<S> {
        let i = self.slot_map.indices_of(id);
        unsafe {
            let actor : &Actor<S> = transmute(
                self.actors.collections[i.collection()].at(i.slot())
            );
            actor
        }
    }

    pub fn at_mut(&mut self, id: usize) -> &mut Actor<S> {
        let i = self.slot_map.indices_of(id);
        unsafe {
            let actor : &mut Actor<S> = transmute(
                self.actors.collections[i.collection()].at_mut(i.slot())
            );
            actor
        }
    }

    pub fn add(&mut self, actor: &Actor<S>) {
        let size = actor.total_size_bytes();
        let collection_index = self.actors.size_to_index(size);
        let ref mut collection = self.actors.sized_for_mut(size);
        let (ptr, index) = collection.push();

        self.slot_map.associate(actor.id.instance_id as usize, SlotIndices::new(collection_index, index));

        unsafe {
            let actor_in_slot : &mut Actor<S> = transmute(ptr);
            actor_in_slot.embed_behind_from(&actor);
        }
    }

    // TODO: what if there is only one actor left??
    fn swap_remove(&mut self, indices: SlotIndices) {
        unsafe {
            let ref mut collection = self.actors.collections[indices.collection()];
            let swapped_actor : &Actor<S> = transmute(collection.swap_remove(indices.slot()));
            self.slot_map.associate(swapped_actor.id.instance_id as usize, indices);
        }
    }

    pub fn remove(&mut self, id: usize) {
        let i = *self.slot_map.indices_of(id);
        self.swap_remove(i);
        self.slot_map.free(id);
    }

    pub fn resize(&mut self, id: usize) {
            let old_i = *self.slot_map.indices_of(id);
            let old_actor_ptr = self.at(id) as *const Actor<S>;
            unsafe {
                self.add(&*old_actor_ptr);
            }
            self.swap_remove(old_i);
    }

    pub fn receive<M: Message>(&mut self, id: usize, message: &M) where Actor<S>: Recipient<M> {
        let is_still_embedded = {
            let actor = self.at_mut(id);
            actor.receive(message);
            actor.is_still_embedded()
        };

        if !is_still_embedded {
            self.resize(id);
        }
    }
}