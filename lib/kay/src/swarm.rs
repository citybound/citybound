//! Tools for dealing with large amounts of identical actors

use super::chunked::{MemChunker, ValueInChunk, SizedChunkedArena, MultiSized};
use super::compact::Compact;
use super::slot_map::{SlotIndices, SlotMap};
use super::messaging::{Recipient, Message, Packet, Fate};
use super::actor_system::Actor;
use super::id::ID;
use std::marker::PhantomData;
use std::mem::size_of;

/// Trait that allows dynamically sized `SubActors` to provide
/// a "typical size" hint to optimize their storage in a `Swarm`
pub trait StorageAware: Sized {
    /// The default implementation just returns the static size of the implementing type
    fn typical_size() -> usize {
        let size = size_of::<Self>();
        if size == 0 { 1 } else { size }
    }
}
impl<T> StorageAware for T {}

/// Trait that sub-actors of a `Swarm` have to implement in order for
/// the Swarm to set the sub-actor id and the sub-actor to get its own id in a standardized way.
///
/// Furthermore, a `SubActor` has to implement [`Compact`](../../compact), so a `Swarm`
/// can compactly store each `SubActor`'s potentially dynamically-sized state.
///
/// This trait can be auto-derived on structs that contain a field `_id: ID` using
/// [`kay_macros`](../../kay_macros/index.html)
pub trait SubActor: Compact + StorageAware + 'static {
    /// Get the full ID (Swarm type id + sub-actor id) of `self`
    fn id(&self) -> ID;
    /// Set the full ID (Swarm type id + sub-actor id) of `self` (called internally by `Swarm`)
    unsafe fn set_id(&mut self, id: ID);
}

fn broadcast_sub_actor_id() -> u32 {
    u32::max_value()
}

/// Offers efficient storage and updating of large numbers of identical `SubActor`s
/// and is typically used whenever there is more than one actor with the same type/behaviour.
///
/// If `SubActor` can receive `Message`, then `Swarm<SubActor>` can do so as well,
/// redirecting each such message to the correct sub-actor by default
/// (see [`RecipientAsSwarm`](trait.RecipientAsSwarm.html))
pub struct Swarm<SubActor> {
    sub_actors: MultiSized<SizedChunkedArena>,
    slot_map: SlotMap,
    n_sub_actors: ValueInChunk<usize>,
    _marker: PhantomData<[SubActor]>,
}

const CHUNK_SIZE: usize = 4096 * 4096 * 4;

impl<SA: SubActor> Swarm<SA> {
    /// Create an empty `Swarm`
    pub fn new() -> Self {
        let chunker = MemChunker::from_settings("", CHUNK_SIZE);
        Swarm {
            sub_actors: MultiSized::new(chunker.child("_sub_actors"), SA::typical_size()),
            n_sub_actors: ValueInChunk::new(chunker.child("_n_sub_actors"), 0),
            slot_map: SlotMap::new(chunker.child("_slot_map")),
            _marker: PhantomData,
        }
    }

    fn allocate_sub_actor_id(&mut self) -> (usize, usize) {
        self.slot_map.allocate_id()
    }

    fn at_index(&self, index: SlotIndices) -> &SA {
        unsafe { &*(self.sub_actors.bins[index.bin()].at(index.slot()) as *const SA) }
    }

    fn at_index_mut(&mut self, index: SlotIndices) -> &mut SA {
        unsafe { &mut *(self.sub_actors.bins[index.bin()].at_mut(index.slot()) as *mut SA) }
    }

    fn at_mut(&mut self, id: usize) -> &mut SA {
        let index = *self.slot_map.indices_of(id);
        self.at_index_mut(index)
    }

    fn add(&mut self, initial_state: &SA) -> ID {
        let (sub_actor_id, version) = self.allocate_sub_actor_id();
        let id = ID::new(unsafe { (*super::THE_SYSTEM).short_id::<Self>() },
                         sub_actor_id as u32,
                         version as u8);
        self.add_with_id(initial_state, id);
        *self.n_sub_actors += 1;
        id
    }

    fn add_with_id(&mut self, initial_state: &SA, id: ID) {
        let size = initial_state.total_size_bytes();
        let bin_index = self.sub_actors.size_to_index(size);
        let bin = &mut self.sub_actors.bin_for_size_mut(size);
        let (ptr, index) = bin.push();

        self.slot_map
            .associate(id.sub_actor_id as usize, SlotIndices::new(bin_index, index));
        assert_eq!(self.slot_map.indices_of(id.sub_actor_id as usize).bin(),
                   bin_index);

        unsafe {
            let actor_in_slot = &mut *(ptr as *mut SA);
            actor_in_slot.compact_behind_from(initial_state);
            actor_in_slot.set_id(id)
        }
    }

    fn swap_remove(&mut self, indices: SlotIndices) -> bool {
        unsafe {
            let bin = &mut self.sub_actors.bins[indices.bin()];
            match bin.swap_remove(indices.slot()) {
                Some(ptr) => {
                    let swapped_actor = &*(ptr as *mut SA);
                    self.slot_map
                        .associate(swapped_actor.id().sub_actor_id as usize, indices);
                    true
                }
                None => false,
            }

        }
    }

    fn remove(&mut self, id: ID) {
        let i = *self.slot_map.indices_of(id.sub_actor_id as usize);
        self.remove_at_index(i, id);
    }

    fn remove_at_index(&mut self, i: SlotIndices, id: ID) {
        self.swap_remove(i);
        self.slot_map
            .free(id.sub_actor_id as usize, id.version as usize);
        *self.n_sub_actors -= 1;
    }

    fn resize(&mut self, id: usize) -> bool {
        let index = *self.slot_map.indices_of(id);
        self.resize_at_index(index)
    }

    fn resize_at_index(&mut self, old_i: SlotIndices) -> bool {
        let old_actor_ptr = self.at_index(old_i) as *const SA;
        let old_actor = unsafe { &*old_actor_ptr };
        self.add_with_id(old_actor, old_actor.id());
        self.swap_remove(old_i)
    }

    fn receive_instance<M: Message>(&mut self, packet: &Packet<M>)
        where SA: Recipient<M>
    {
        let (fate, is_still_compact) = {
            let actor = self.at_mut(packet
                                        .recipient_id
                                        .expect("Recipient ID not set")
                                        .sub_actor_id as usize);
            let fate = actor.receive_packet(packet);
            (fate, actor.is_still_compact())
        };

        match fate {
            Fate::Live => {
                if !is_still_compact {
                    self.resize(packet
                                    .recipient_id
                                    .expect("Recipient ID not set")
                                    .sub_actor_id as usize);
                }
            }
            Fate::Die => self.remove(packet.recipient_id.expect("Recipient ID not set")),
        }
    }

    fn receive_broadcast<M: Message>(&mut self, packet: &Packet<M>)
        where SA: Recipient<M>
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
            self.sub_actors
                .bins
                .iter()
                .map(|bin| bin.len())
                .collect()
        };

        let n_bins = self.sub_actors.bins.len();

        for (c, recipients_todo) in recipients_todo_per_bin.iter().enumerate().take(n_bins) {
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
                    Fate::Live => {
                        if is_still_compact {
                            false
                        } else {
                            self.resize_at_index(index);
                            // this should also work in the case where the "resized" actor
                            // itself is added to the same bin again
                            let swapped_in_another_receiver = self.sub_actors.bins[c].len() <
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
                        let swapped_in_another_receiver = self.sub_actors.bins[c].len() <
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

    /// Get an ID that refers to all sub-actors inside a `Swarm`.
    /// If you send a message to this ID, all sub-actors will receive it
    /// (Exception: the Swarm handles the message itself, see
    /// [`RecipientAsSwarm`](trait.RecipientAsSwarm.html))
    pub fn all() -> ID
        where Self: Sized
    {
        ID::new(unsafe { (*super::THE_SYSTEM).short_id::<Self>() },
                broadcast_sub_actor_id(),
                0)
    }
}

impl<SA: SubActor> Default for Swarm<SA> {
    fn default() -> Self {
        Self::new()
    }
}

/// A `Swarm` is itself an `Actor`!
impl<SA: SubActor> Actor for Swarm<SA> {}

/// Helper trait for dispatching messages either to `SubActor`s of a `Swarm` or the `Swarm` itself.
///
/// By implementing this trait on a `SubActor`, you indirectly control
/// what `Swarm<SubActor>` will be a `Recipient` of.
/// *This is required since you can't implement `Recipient` on `Swarm` yourself,
/// both being foreign types.*
///
/// The default implementation will make `Swarm<SubActor>` a `Recipient<Message>` for all messages
/// that `SubActor` is a `Recipient` of, dispatching incoming messages to the correct sub-actor
/// based on the sub-actor id part of the `Packet`'s recipient field.
///
/// To override this behaviour and handle all messages of a certain type on the Swarm level,
/// you can implement `RecipientAsSwarm<MessageForSwarm>` for `SubActor` yourself.
///
/// Further, already given implementations define how a `Swarm` handles
/// `RequestConfirmation`, `ToRandom`, `Create` and `CreateWith` messages.
pub trait RecipientAsSwarm<M: Message>: Sized {
    /// Analogous to [`Recipient::receive`](trait.Recipient.html#method.receive)
    fn receive(_swarm: &mut Swarm<Self>, _message: &M) -> Fate {
        unimplemented!()
    }
    /// Analogous to [`Recipient::receive_packet`](trait.Recipient.html#method.receive_packet)
    fn receive_packet(swarm: &mut Swarm<Self>, packet: &Packet<M>) -> Fate {
        Self::receive(swarm, &packet.message)
    }
}

/// See [`RecipientAsSwarm`](trait.RecipientAsSwarm.html)
impl<M: Message, SA: SubActor + RecipientAsSwarm<M>> Recipient<M> for Swarm<SA> {
    fn receive_packet(&mut self, packet: &Packet<M>) -> Fate {
        SA::receive_packet(self, packet)
    }
}

/// Default implementation that redirects messages to sub-actors
impl<M: Message + NotACreateMessage + NotARequestConfirmationMessage + NotAToRandomMessage,
     SA: SubActor + Recipient<M>> RecipientAsSwarm<M>
    for SA {
    fn receive_packet(swarm: &mut Swarm<SA>, packet: &Packet<M>) -> Fate {
        if packet
               .recipient_id
               .expect("Recipient ID not set")
               .sub_actor_id == broadcast_sub_actor_id() {
            swarm.receive_broadcast(packet);
        } else {
            swarm.receive_instance(packet);
        }
        Fate::Live
    }
}

/// A wrapper for messages to request a confirmation about message receival.
///
/// The receiving `Swarm` will reply to `requester` with a
/// [`Confirmation`](struct.Confirmation.html).
///
/// This is typically used to make sure that all sub-actors
/// of a certain kind received and handled a message.
///
/// Note: although the handler is implemented in `Swarm`, you need to call
/// `Swarm::<SubActor>::handle::<RequestConfirmation<Message>>();`
/// to actually register the handler (for each message type that should be confirmable).
#[derive(Compact, Clone)]
pub struct RequestConfirmation<M: Message> {
    /// Who should the `Confirmation` reply be sent to?
    pub requester: ID,
    /// Actual message that should be handled
    pub message: M,
}

#[doc(hidden)]
pub trait NotARequestConfirmationMessage {}
impl NotARequestConfirmationMessage for .. {}
impl<M: Message> !NotARequestConfirmationMessage for RequestConfirmation<M> {}


/// The reply to a [`RequestConfirmation`](struct.RequestConfirmation.html)
#[derive(Clone)]
pub struct Confirmation<M: Message> {
    /// How many sub-actors actually received and handled the message
    pub n_recipients: usize,
    _marker: PhantomData<*const M>,
}

impl<M: Message> Copy for Confirmation<M> {}

impl<M: Message, SA: SubActor + RecipientAsSwarm<M>> RecipientAsSwarm<RequestConfirmation<M>>
    for SA {
    fn receive_packet(swarm: &mut Swarm<SA>, packet: &Packet<RequestConfirmation<M>>) -> Fate {
        let n_recipients = if packet
               .recipient_id
               .expect("Recipient ID not set")
               .sub_actor_id == broadcast_sub_actor_id() {
            *swarm.n_sub_actors
        } else {
            1
        };
        let fate = SA::receive_packet(swarm,
                                      &Packet {
                                           recipient_id: packet.recipient_id,
                                           message: packet.message.message.clone(),
                                       });
        packet.message.requester <<
        Confirmation::<M> {
            n_recipients: n_recipients,
            _marker: PhantomData,
        };
        fate
    }
}

/// A wrapper for messages to send a message to random sub-actors.
///
/// Note: although the handler is implemented in `Swarm`, you need to call
/// `Swarm::<SubActor>::handle::<ToRandom<Message>>();`
/// to actually register the handler (for each message type that will be sent randomly).
#[derive(Compact, Clone)]
pub struct ToRandom<M: Message> {
    /// Actual message that should be handled
    pub message: M,
    /// Number of randomly selected sub-actors that will receive the message
    pub n_recipients: usize,
}

#[doc(hidden)]
pub trait NotAToRandomMessage {}
impl NotAToRandomMessage for .. {}
impl<M: Message> !NotAToRandomMessage for ToRandom<M> {}

impl<M: Message, SA: SubActor + RecipientAsSwarm<M>> RecipientAsSwarm<ToRandom<M>> for SA {
    fn receive_packet(swarm: &mut Swarm<SA>, packet: &Packet<ToRandom<M>>) -> Fate {
        if swarm.slot_map.len() > 0 {
            let mut new_packet = Packet {
                recipient_id: None,
                message: packet.message.message.clone(),
            };
            for _i in 0..packet.message.n_recipients {
                let random_id = ID::new(unsafe { (*super::THE_SYSTEM).short_id::<Swarm<SA>>() },
                                        swarm.slot_map.random_used() as u32,
                                        0);
                new_packet.recipient_id = Some(random_id);
                swarm.receive_packet(&new_packet);
            }
        }
        Fate::Live
    }
}

/// A message for adding a new sub-actor to a `Swarm` given its initial state.
///
/// Note: although the handler is implemented in `Swarm`, you need to call
/// `Swarm::<SubActor>::handle::<Create<SubActor>>();`
/// to actually register the handler for creating sub-actors like this.
#[derive(Compact, Clone)]
pub struct Create<SA: SubActor>(pub SA);

#[doc(hidden)]
pub trait NotACreateMessage {}

impl NotACreateMessage for .. {}

impl<SA: SubActor> !NotACreateMessage for Create<SA> {}
impl<SA: SubActor, M: Message> !NotACreateMessage for CreateWith<SA, M> {}

impl<SA: SubActor> RecipientAsSwarm<Create<SA>> for SA {
    fn receive(swarm: &mut Swarm<SA>, msg: &Create<SA>) -> Fate {
        match *msg {
            Create(ref initial_state) => {
                swarm.add(initial_state);
                Fate::Live
            }
        }
    }
}

/// A message for adding a new sub-actor to a `Swarm` given its initial state
/// and an initial message that the sub-actor will handle immediately after creation.
///
/// Note: although the handler is implemented in `Swarm`, you need to call
/// `Swarm::<SubActor>::handle::<CreateWith<SubActor, Message>>();`
/// to actually register the handler for creating sub-actors like this.
#[derive(Compact, Clone)]
pub struct CreateWith<SA: SubActor, M: Message>(pub SA, pub M);

impl<M: Message, SA: SubActor + Recipient<M>> RecipientAsSwarm<CreateWith<SA, M>> for SA {
    fn receive(swarm: &mut Swarm<SA>, msg: &CreateWith<SA, M>) -> Fate {
        match *msg {
            CreateWith(ref initial_state, ref initial_message) => {
                let id = swarm.add(initial_state);
                let initial_packet = Packet {
                    recipient_id: Some(id),
                    message: (*initial_message).clone(),
                };
                swarm.receive_instance(&initial_packet);
                Fate::Live
            }
        }
    }
}
