//! Tools for dealing with large amounts of identical actors

use super::chunked::{MemChunker, ValueInChunk, SizedChunkedArena, MultiSized};
use super::compact::Compact;
use super::slot_map::{SlotIndices, SlotMap};
use super::messaging::{Message, Packet, Fate};
use super::actor_system::{World, ActorDefiner};
use super::id::{ID, broadcast_sub_actor_id};
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

/// Trait that sub-actors of a [`Swarm`](struct.Swarm.html) have to implement
/// so their internally stored sub-actor ID can be gotten and set.
///
/// Furthermore, a `SubActor` has to implement [`Compact`](../../compact), so a `Swarm`
/// can compactly store each `SubActor`'s potentially dynamically-sized state.
///
/// This trait can be auto-derived on structs that contain a field `_id: Option<ID>` using
/// [`kay_macros`](../../kay_macros/index.html)
pub trait SubActor: Compact + StorageAware + 'static {
    /// Get the full ID (Swarm type id + sub-actor id) of `self`
    fn id(&self) -> ID;
    /// Set the full ID (Swarm type id + sub-actor id) of `self` (called internally by `Swarm`)
    unsafe fn set_id(&mut self, id: ID);
}

/// A container-like actor, housing many sub-actors of identical behaviour.
///
/// Offers efficient storage of and broadcasting to its sub-actors.
///
/// New sub-actors can be added to a swarm using [`Create`](struct.Create.html)
/// or [`CreateWith`](struct.CreateWith.html).
pub struct Swarm<SubActor> {
    sub_actors: MultiSized<SizedChunkedArena>,
    slot_map: SlotMap,
    n_sub_actors: ValueInChunk<usize>,
    _marker: PhantomData<[SubActor]>,
}

const CHUNK_SIZE: usize = 4096 * 4096 * 4;

impl<SA: SubActor + Clone> Swarm<SA> {
    /// Create an empty `Swarm`.
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

    fn at_index_mut(&mut self, index: SlotIndices) -> &mut SA {
        unsafe { &mut *(self.sub_actors.bins[index.bin()].at_mut(index.slot()) as *mut SA) }
    }

    fn at_mut(&mut self, id: usize) -> &mut SA {
        let index = *self.slot_map.indices_of(id);
        self.at_index_mut(index)
    }

    /// Allocate a subactor ID for later use when manually adding a subactor (see `add_with_id`)
    pub unsafe fn allocate_id(&mut self, base_id: ID) -> ID {
        let (sub_actor_id, version) = self.allocate_sub_actor_id();
        ID::new(base_id.type_id, sub_actor_id as u32, version as u8)
    }

    fn add(&mut self, mut initial_state: SA, base_id: ID) -> ID {
        let id = unsafe { self.allocate_id(base_id) };
        unsafe { self.add_with_id(&mut initial_state, id) };
        *self.n_sub_actors += 1;
        // this acts like a move!
        ::std::mem::forget(initial_state);
        id
    }

    /// used externally when manually adding a subactor,
    /// making use of a previously allocated ID (see `allocate_id`)
    pub unsafe fn add_manually_with_id(&mut self, initial_state: *mut SA, id: ID) {
        self.add_with_id(initial_state, id);
        *self.n_sub_actors += 1;
    }

    /// Used internally
    unsafe fn add_with_id(&mut self, initial_state: *mut SA, id: ID) {
        let size = (*initial_state).total_size_bytes();
        let bin_index = self.sub_actors.size_to_index(size);
        let bin = &mut self.sub_actors.bin_for_size_mut(size);
        let (ptr, index) = bin.push();

        self.slot_map.associate(
            id.sub_actor_id as usize,
            SlotIndices::new(bin_index, index),
        );
        assert_eq!(
            self.slot_map.indices_of(id.sub_actor_id as usize).bin(),
            bin_index
        );

        Compact::compact_behind(initial_state, ptr as *mut SA);
        let actor_in_slot = &mut *(ptr as *mut SA);
        actor_in_slot.set_id(id);
    }

    fn swap_remove(&mut self, indices: SlotIndices) -> bool {
        unsafe {
            let bin = &mut self.sub_actors.bins[indices.bin()];
            match bin.swap_remove(indices.slot()) {
                Some(ptr) => {
                    let swapped_actor = &*(ptr as *mut SA);
                    self.slot_map.associate(
                        swapped_actor.id().sub_actor_id as usize,
                        indices,
                    );
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
        // TODO: not sure if this is the best place to drop actor state
        let old_actor_ptr = self.at_index_mut(i) as *mut SA;
        unsafe {
            ::std::ptr::drop_in_place(old_actor_ptr);
        }
        self.swap_remove(i);
        self.slot_map.free(
            id.sub_actor_id as usize,
            id.version as usize,
        );
        *self.n_sub_actors -= 1;
    }

    fn resize(&mut self, id: usize) -> bool {
        let index = *self.slot_map.indices_of(id);
        self.resize_at_index(index)
    }

    fn resize_at_index(&mut self, old_i: SlotIndices) -> bool {
        let old_actor_ptr = self.at_index_mut(old_i) as *mut SA;
        unsafe { self.add_with_id(old_actor_ptr, (*old_actor_ptr).id()) };
        self.swap_remove(old_i)
    }

    fn receive_instance<M: Message, H>(
        &mut self,
        packet: &Packet<M>,
        handler: &H,
        world: &mut World,
    ) where
        H: Fn(&Packet<M>, &mut SA, &mut World) -> Fate + 'static,
    {
        let (fate, is_still_compact) = {
            let actor = self.at_mut(
                packet
                    .recipient_id
                    .expect("Recipient ID not set")
                    .sub_actor_id as usize,
            );
            let fate = handler(packet, actor, world);
            (fate, actor.is_still_compact())
        };

        match fate {
            Fate::Live => {
                if !is_still_compact {
                    self.resize(
                        packet
                            .recipient_id
                            .expect("Recipient ID not set")
                            .sub_actor_id as usize,
                    );
                }
            }
            Fate::Die => self.remove(packet.recipient_id.expect("Recipient ID not set")),
        }
    }

    fn receive_broadcast<M: Message, H>(
        &mut self,
        packet: &Packet<M>,
        handler: &H,
        world: &mut World,
    ) where
        H: Fn(&Packet<M>, &mut SA, &mut World) -> Fate + 'static,
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
            self.sub_actors.bins.iter().map(|bin| bin.len()).collect()
        };

        let n_bins = self.sub_actors.bins.len();

        for (c, recipients_todo) in recipients_todo_per_bin.iter().enumerate().take(n_bins) {
            let mut slot = 0;
            let mut index_after_last_recipient = *recipients_todo;

            for _ in 0..*recipients_todo {
                let index = SlotIndices::new(c, slot);
                let (fate, is_still_compact, id) = {
                    let actor = self.at_index_mut(index);
                    let fate = handler(packet, actor, world);
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

    /// Define message handlers for subactors of a `Swarm` when
    /// it is added to a system or extended. See
    /// [`SubActorDefiner`](struct.SubActorDefiner.html) to see what can be defined.
    ///
    /// ```
    /// system.add(Swarm::<UIElement>::new(), Swarm::<UI>::subactors(|mut each_element| {
    ///    each_element.on(|&Update, element, world| {
    ///        //...
    ///    });
    /// }));
    pub fn subactors<S>(subactor_definition: S) -> impl Fn(ActorDefiner<Self>)
    where
        S: Fn(SubActorDefiner<SA>) + 'static,
    {
        move |mut the_swarm| {
            Self::define_control_handlers(&mut the_swarm);
            subactor_definition(SubActorDefiner(the_swarm));
        }
    }

    fn define_control_handlers<'a>(the_swarm: &mut ActorDefiner<'a, Self>) {
        let swarm_id = the_swarm.world().id::<Self>();
        the_swarm.on(move |&Create(ref initial_state): &Create<SA>, swarm, _| {
            swarm.add(initial_state.clone(), swarm_id);
            Fate::Live
        });
    }
}

use super::actor_system::SubActorsCountable;
impl<SA: SubActor> SubActorsCountable for Swarm<SA> {
    fn subactor_count(&self) -> usize {
        *self.n_sub_actors
    }
}

/// A message for adding a new sub-actor to a `Swarm` given its initial state.
#[derive(Compact, Clone)]
pub struct Create<SA: SubActor>(pub SA);

/// A message for adding a new sub-actor to a `Swarm` given its initial state
/// and an initial message that the sub-actor will handle immediately after creation.
///
/// Note: this requires that a message handler was defined for the sub-actors
/// using [`on_create_with`](struct.SubActorDefiner.html#method.on_create_with).
#[derive(Compact, Clone)]
pub struct CreateWith<SA: SubActor, M: Message>(pub SA, pub M);

/// A wrapper for messages to send a message to a random
/// subset of the sub-actors of a `Swarm`.
///
/// Note: this requires that a message handler was defined for the sub-actors
/// using [`on_random`](struct.SubActorDefiner.html#method.on_random).
#[derive(Compact, Clone)]
pub struct ToRandom<M: Message> {
    /// Actual message that should be handled
    pub message: M,
    /// Number of randomly selected sub-actors that will receive the message
    pub n_recipients: usize,
}

/// Helper that is used to define behaviour (message handlers) of sub-actors in a `Swarm`.
/// Analogous to [`ActorDefiner`](../struct.ActorDefiner.html)
/// but with some `Swarm`-related extras.
///
/// It is passed to the closure argument of
/// [`Swarm::subactors`](struct.Swarm.html#method.subactors).
pub struct SubActorDefiner<'a, SA: 'static>(ActorDefiner<'a, Swarm<SA>>);

impl<'a, SA: SubActor + Clone + 'static> SubActorDefiner<'a, SA> {
    /// Analogous to [`ActorDefiner::on`](../struct.ActorDefiner.html#method.on),
    /// the closure argument is passed:
    ///
    /// * the received message (can conveniently be destructured already as an argument)
    /// * the current **sub-actor state** that can be mutated
    /// * a [`World`](struct.World.html) to identify and send
    ///   messages to other actors
    pub fn on<M: Message, F>(&mut self, handler: F)
    where
        F: Fn(&M, &mut SA, &mut World) -> Fate + 'static,
    {
        self.on_maybe_critical(handler, false);
    }

    /// Same as [`on`](#method.on) but continues to receive after the ActorSystem panicked.
    /// (Analogous to [`ActorDefiner::on_critical`](../struct.ActorDefiner.html#method.on_critical))
    pub fn on_critical<M: Message, F>(&mut self, handler: F)
    where
        F: Fn(&M, &mut SA, &mut World) -> Fate + 'static,
    {
        self.on_maybe_critical(handler, true);
    }

    /// Allows sub-actors to be created with a particular message
    /// (the one handled by the given closure) as an initial message.
    ///
    /// This can then be triggered by sending a [`CreateWith`](struct.CreateWith.html)
    /// message to the swarm.
    pub fn on_create_with<M: Message + Clone, F>(&mut self, handler: F)
    where
        F: Fn(&M, &mut SA, &mut World) -> Fate + 'static,
    {
        self.0.on_packet(
            move |&Packet { ref message, recipient_id }, swarm, world| {
                let &CreateWith(ref init_state, ref init_message): &CreateWith<
                    SA,
                    M,
                > = message;
                let id = swarm.add(init_state.clone(), recipient_id.unwrap());
                world.send(id, (*init_message).clone());
                Fate::Live
            },
            false,
        );

        // also be able to receive this message normally
        self.on(handler);
    }

    /// Allows a message (the one handled by the given closure) to be received
    /// by a random subset of the sub-actors of a `Swarm`.
    ///
    /// This can then be triggered by sending a [`ToRandom`](struct.ToRandom.html)
    /// message to the swarm.
    pub fn on_random<M: Message + Clone, F>(&mut self, handler: F)
    where
        F: Fn(&M, &mut SA, &mut World) -> Fate + 'static,
    {
        self.0.on_packet(
            move |packet: &Packet<ToRandom<M>>, swarm, world| {
                if swarm.slot_map.len() > 0 {
                    for _i in 0..packet.message.n_recipients {
                        let random_id = ID::new(
                            packet.recipient_id.unwrap().type_id,
                            swarm.slot_map.random_used() as u32,
                            0,
                        );
                        world.send(random_id, packet.message.message.clone());
                    }
                }
                Fate::Live
            },
            false,
        );

        // also be able to receive this message normally
        self.on(handler);
    }

    /// Access a [`World`](struct.World.html) of the system that a sub-actor
    /// is being defined in, can be used to identify actors (and keep the ID)
    /// or send messages at *define-time*.
    ///
    /// (Analogous to [`ActorDefiner::world`](../struct.ActorDefiner.html#method.world))
    pub fn world(&mut self) -> World {
        self.0.world()
    }

    /// Advanced: Can be used to register a handler not only for a message,
    /// but for a whole packet (precise recipient id + message).
    ///
    /// (Analogous to [`ActorDefiner::on_packet`](../struct.ActorDefiner.html#method.on_packet))
    pub fn on_packet<M: Message, F>(&mut self, handler: F, critical: bool)
    where
        F: Fn(&Packet<M>, &mut SA, &mut World) -> Fate + 'static,
    {
        self.0.on_packet(
            move |packet: &Packet<M>, swarm, world| {
                if packet
                    .recipient_id
                    .expect("Recipient ID not set")
                    .sub_actor_id == broadcast_sub_actor_id()
                {
                    swarm.receive_broadcast(packet, &handler, world);
                } else {
                    swarm.receive_instance(packet, &handler, world);
                }
                Fate::Live
            },
            critical,
        );


    }

    fn on_maybe_critical<M: Message, F>(&mut self, handler: F, critical: bool)
    where
        F: Fn(&M, &mut SA, &mut World) -> Fate + 'static,
    {
        self.on_packet(
            move |packet: &Packet<M>, state, world| handler(&packet.message, state, world),
            critical,
        );
    }
}
