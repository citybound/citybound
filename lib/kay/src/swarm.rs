use super::compact::{Compact};
use super::chunked::{MemChunker, ValueInChunk, SizedChunkedArena, MultiSized};
use super::slot_map::{SlotIndices, SlotMap};
use super::messaging::{Actor, Individual, Recipient, Message, Packet, Fate};
use super::actor_system::{ID};
use ::std::marker::PhantomData;

pub struct Swarm<Actor> {
    actors: MultiSized<SizedChunkedArena>,
    slot_map: SlotMap,
    n_actors: ValueInChunk<usize>,
    _marker: PhantomData<[Actor]>
}

const CHUNK_SIZE : usize = 4096 * 128;

impl<A: Actor> Swarm<A> {
    pub fn new() -> Self {
        let chunker = MemChunker::new("", CHUNK_SIZE);
        Swarm{
            actors: MultiSized::new(chunker.child("_actors"), A::typical_size()),
            n_actors: ValueInChunk::new(chunker.child("_n_actors"), 0),
            slot_map: SlotMap::new(chunker.child("_slot_map")),
            _marker: PhantomData
        }
    }

    fn allocate_instance_id(&mut self) -> (usize, usize) {
        self.slot_map.allocate_id()
    }

    fn at_index(&self, index: SlotIndices) -> &A {
        unsafe {&*(self.actors.collections[index.collection()].at(index.slot()) as *const A)}
    }

    fn at_index_mut(&mut self, index: SlotIndices) -> &mut A {
        unsafe {&mut *(self.actors.collections[index.collection()].at_mut(index.slot()) as *mut A)}
    }

    fn at_mut(&mut self, id: usize) -> &mut A {
        let index = *self.slot_map.indices_of(id);
        self.at_index_mut(index)
    }

    fn add(&mut self, initial_state: &A) -> ID {
        let id = unsafe{(*super::actor_system::THE_SYSTEM).instance_id::<A>(self.allocate_instance_id())};
        self.add_with_id(initial_state, id);
        *self.n_actors += 1;
        id
    }

    fn add_with_id(&mut self, initial_state: &A, id: ID) {
        let size = initial_state.total_size_bytes();
        let collection_index = self.actors.size_to_index(size);
        let collection = &mut self.actors.sized_for_mut(size);
        let (ptr, index) = collection.push();

        self.slot_map.associate(id.instance_id as usize, SlotIndices::new(collection_index, index));
        assert!(self.slot_map.indices_of(id.instance_id as usize).collection() == collection_index);

        unsafe {
            let actor_in_slot = &mut *(ptr as *mut A);
            actor_in_slot.compact_behind_from(initial_state);
            actor_in_slot.set_id(id)
        }
    }

    fn swap_remove(&mut self, indices: SlotIndices) -> bool {
        unsafe {
            let collection = &mut self.actors.collections[indices.collection()];
            match collection.swap_remove(indices.slot()) {
                Some(ptr) => {
                    let swapped_actor = &*(ptr as *mut A);
                    self.slot_map.associate(swapped_actor.id().instance_id as usize, indices);
                    true
                },
                None => false
            }
            
        }
    }

    fn remove(&mut self, id: ID) {
        let i = *self.slot_map.indices_of(id.instance_id as usize);
        self.remove_at_index(i, id);
    }

    fn remove_at_index(&mut self, i: SlotIndices, id: ID) {
        self.swap_remove(i);
        self.slot_map.free(id.instance_id as usize, id.version as usize);
        *self.n_actors -= 1;
    }

    fn resize(&mut self, id: usize) -> bool {
        let index = *self.slot_map.indices_of(id);
        self.resize_at_index(index)
    }

    fn resize_at_index(&mut self, old_i: SlotIndices) -> bool {
        let old_actor_ptr = self.at_index(old_i) as *const A;
        let old_actor = unsafe{&*old_actor_ptr};
        self.add_with_id(old_actor, old_actor.id());
        self.swap_remove(old_i)
    }

    fn receive_instance<M: Message>(&mut self, packet: &Packet<M>) where A: Recipient<M> {
        let (fate, is_still_compact) = {
            let actor = self.at_mut(packet.recipient_id.instance_id as usize);
            let fate = actor.receive_packet(packet);
            (fate, actor.is_still_compact())
        };

        match fate {
            Fate::Live => {
                if !is_still_compact {
                    self.resize(packet.recipient_id.instance_id as usize);
                }
            },
            Fate::Die | Fate::Explode(..) => self.remove(packet.recipient_id)
        }
    }

    fn receive_broadcast<M: Message>(&mut self, packet: &Packet<M>) where A: Recipient<M> {
        // this function has to deal with the fact that during the iteration, receivers of the broadcast can be resized
        // and thus removed from a collection, swapping in either
        //    - other receivers that didn't receive the broadcast yet
        //    - resized and added receivers that alredy received the broadcast
        //    - actors that were created during one of the broadcast receive handlers, that shouldn't receive this broadcast
        // the only assumption is that no actors are immediately completely deleted

        let recipients_todo_per_collection : Vec<usize> = {
            self.actors.collections.iter().map(|collection| {collection.len()}).collect()
        };

        let n_collections = self.actors.collections.len();

        for (c, recipients_todo) in recipients_todo_per_collection.iter().enumerate().take(n_collections) {
            let mut slot = 0;
            let mut index_after_last_recipient = *recipients_todo;

            for _ in 0..*recipients_todo {
                let index = SlotIndices::new(c, slot);
                let (fate, is_still_compact, id) = {
                    let actor = self.at_index_mut(index);
                    let fate = actor.receive_packet(packet);
                    (fate, actor.is_still_compact(), actor.id())
                };

                let repeat_slot = match fate {
                    Fate::Live => if is_still_compact {
                            false
                        } else {
                            self.resize_at_index(index);
                            // this should also work in the case where the "resized" actor itself is added to the same collection again
                            let swapped_in_another_receiver = self.actors.collections[c].len() < index_after_last_recipient;
                            if swapped_in_another_receiver {
                                index_after_last_recipient -= 1;
                                true
                            } else {
                                false
                            }
                        },
                    Fate::Die | Fate::Explode(..) => {
                        self.remove_at_index(index, id);
                        // this should also work in the case where the "resized" actor itself is added to the same collection again
                        let swapped_in_another_receiver = self.actors.collections[c].len() < index_after_last_recipient;
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

    pub fn all() -> ID where Self: Sized {
        unsafe{(*super::actor_system::THE_SYSTEM).broadcast_id::<A>()}
    }
}

impl <A: Actor> Individual for Swarm<A> {}

pub trait RecipientAsSwarm<M: Message> : Sized {
    fn receive_packet(swarm: &mut Swarm<Self>, packet: &Packet<M>) -> Fate {
        Self::receive(swarm, &packet.message)
    }
    fn receive(_swarm: &mut Swarm<Self>, _message: &M) -> Fate {unimplemented!()}
}

impl <M: Message, A: Actor + RecipientAsSwarm<M>> Recipient<M> for Swarm<A> {
    fn receive_packet(&mut self, packet: &Packet<M>) -> Fate {
        A::receive_packet(self, packet)
    }
}

impl <M: Message + NotACreateMessage + NotARequestConfirmationMessage, A: Actor + Recipient<M>> RecipientAsSwarm<M> for A {
    fn receive_packet(swarm: &mut Swarm<A>, packet: &Packet<M>) -> Fate {
        if packet.recipient_id.is_broadcast() {
            swarm.receive_broadcast(packet);
        } else {
            swarm.receive_instance(packet);
        }
        Fate::Live
    }
}

#[derive(Clone)]
pub struct RequestConfirmation<M: Message> {
    pub requester: ID,
    pub message: M
}

impl<M: Message> Compact for RequestConfirmation<M> {
    fn is_still_compact(&self) -> bool {self.message.is_still_compact()}
    fn dynamic_size_bytes(&self) -> usize {self.message.dynamic_size_bytes()}
    unsafe fn compact_from(&mut self, source: &Self, new_dynamic_part: *mut u8) {
        self.requester = source.requester;
        self.message.compact_from(&source.message, new_dynamic_part);
    }
    unsafe fn decompact(&self) -> RequestConfirmation<M> {
        RequestConfirmation{
            requester: self.requester,
            message: self.message.decompact()
        }
    }
}

pub trait NotARequestConfirmationMessage {}

impl NotARequestConfirmationMessage for .. {}

impl <M: Message> !NotARequestConfirmationMessage for RequestConfirmation<M> {}

#[derive(Clone)]
pub struct Confirmation<M: Message>{
    pub n_recipients: usize,
    _marker: PhantomData<*const M>
}

impl<M: Message> Copy for Confirmation<M> { }

impl<M: Message, A: Actor + RecipientAsSwarm<M>> RecipientAsSwarm<RequestConfirmation<M>> for A {
    fn receive_packet(swarm: &mut Swarm<A>, packet: &Packet<RequestConfirmation<M>>) -> Fate {
        let n_recipients = if packet.recipient_id.is_broadcast() {
            *swarm.n_actors
        } else {
            1
        };
        let fate = A::receive_packet(swarm, &Packet{
            recipient_id: packet.recipient_id,
            message: packet.message.message.clone()
        });
        packet.message.requester << Confirmation::<M>{
            n_recipients: n_recipients,
            _marker: PhantomData
        };
        fate
    }
}

#[derive(Clone)]
pub struct Create<A: Actor>(pub A);

impl<A: Actor> Compact for Create<A> {
    fn is_still_compact(&self) -> bool {self.0.is_still_compact()}
    fn dynamic_size_bytes(&self) -> usize {self.0.dynamic_size_bytes()}
    unsafe fn compact_from(&mut self, source: &Self, new_dynamic_part: *mut u8) {
        self.0.compact_from(&source.0, new_dynamic_part);
    }
    unsafe fn decompact(&self) -> Create<A> {
        Create(self.0.decompact())
    }
}

pub trait NotACreateMessage {}

impl NotACreateMessage for .. {}

impl <A: Actor> !NotACreateMessage for Create<A> {}
impl <A: Actor, M: Message> !NotACreateMessage for CreateWith<A, M> {}

impl <A: Actor> RecipientAsSwarm<Create<A>> for A {
    fn receive(swarm: &mut Swarm<A>, msg: &Create<A>) -> Fate {match *msg{
        Create(ref initial_state) => {
            swarm.add(initial_state);
            Fate::Live
        }
    }}
}

#[derive(Clone)]
pub struct CreateWith<A: Actor, M: Message>(pub A, pub M);

impl<A: Actor, M: Message> Compact for CreateWith<A, M> {
    fn is_still_compact(&self) -> bool {self.0.is_still_compact() && self.1.is_still_compact()}
    fn dynamic_size_bytes(&self) -> usize {self.0.dynamic_size_bytes() + self.1.dynamic_size_bytes()}
    unsafe fn compact_from(&mut self, source: &Self, new_dynamic_part: *mut u8) {
        self.0.compact_from(&source.0, new_dynamic_part);
        self.1.compact_from(&source.1, new_dynamic_part.offset(self.0.dynamic_size_bytes() as isize))
    }
    unsafe fn decompact(&self) -> CreateWith<A, M> {
        CreateWith(self.0.decompact(), self.1.decompact())
    }
}

impl <M: Message, A: Actor + Recipient<M>> RecipientAsSwarm<CreateWith<A, M>> for A {
    fn receive(swarm: &mut Swarm<A>, msg: &CreateWith<A, M>) -> Fate {match *msg{
        CreateWith(ref initial_state, ref initial_message) => {
            let id = swarm.add(initial_state);
            let initial_packet = Packet{
                recipient_id: id,
                message: (*initial_message).clone()
            };
            swarm.receive_instance(&initial_packet);
            Fate::Live
        }
    }}
}