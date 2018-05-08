use compact::Compact;
use std::mem::size_of;
use super::messaging::{Message, Packet, Fate};
use super::inbox::{Inbox, DispatchablePacket};
use super::id::{RawID, TypedID, MachineID};
use super::type_registry::{ShortTypeId, TypeRegistry};
use super::swarm::Swarm;
use super::networking::Networking;
use std::panic::{AssertUnwindSafe, catch_unwind};

/// Trait that allows dynamically sized `Actor` instances to provide
/// a "typical size" hint to optimize their storage in a `Swarm`
pub trait StorageAware: Sized {
    /// The default implementation just returns the static size of the implementing type
    fn typical_size() -> usize {
        let size = size_of::<Self>();
        if size == 0 { 1 } else { size }
    }
}
impl<T> StorageAware for T {}

/// Trait that Actors instance have to implement for a [`Swarm`](struct.Swarm.html)
/// so their internally stored instance `RawID` can be gotten and set.
///
/// Furthermore, an `Actor` has to implement [`Compact`](../../compact), so a `Swarm`
/// can compactly store each `Actor`'s potentially dynamically-sized state.
///
/// This trait can is auto-derived when using the
/// [`kay_codegen`](../../kay_codegen/index.html) build script.
pub trait Actor: Compact + StorageAware + 'static {
    /// The unique `TypedID` of this actor
    type ID: TypedID;
    /// Get `TypedID` of this actor
    fn id(&self) -> Self::ID;
    /// Set the full RawID (Actor type id + instance id)
    /// of this actor (only used internally by `Swarm`)
    unsafe fn set_id(&mut self, id: RawID);

    /// Get the id of this actor as an actor trait `TypedID`
    /// (available if the actor implements the corresponding trait)
    fn id_as<TargetID: TraitIDFrom<Self>>(&self) -> TargetID {
        TargetID::from(self.id())
    }

    /// Get the `TypedID` of the local first actor of this kind
    fn local_first(world: &mut World) -> Self::ID {
        unsafe { Self::ID::from_raw(world.local_first::<Self>()) }
    }

    /// Get the `TypedID` of the global first actor of this kind
    fn global_first(world: &mut World) -> Self::ID {
        unsafe { Self::ID::from_raw(world.global_first::<Self>()) }
    }

    /// Get the `TypedID` representing a local broadcast to actors of this type
    fn local_broadcast(world: &mut World) -> Self::ID {
        unsafe { Self::ID::from_raw(world.local_broadcast::<Self>()) }
    }

    /// Get the `TypedID` representing a global broadcast to actors of this type
    fn global_broadcast(world: &mut World) -> Self::ID {
        unsafe { Self::ID::from_raw(world.global_broadcast::<Self>()) }
    }
}

/// Helper trait that signifies that an actor's `TypedID` can be converted
/// to an actor trait `TypedID` if that actor implements the corresponding trait.
pub trait TraitIDFrom<A: Actor>: TypedID {
    /// Construct the actor trait `TypedID` from an actor's `TypedID`
    fn from(id: <A as Actor>::ID) -> Self {
        unsafe { Self::from_raw(id.as_raw()) }
    }
}

struct Dispatcher {
    function: Box<Fn(*const (), &mut World)>,
    critical: bool,
}

const MAX_RECIPIENT_TYPES: usize = 64;
const MAX_MESSAGE_TYPES: usize = 256;

/// The main thing inside of which all the magic happens.
///
/// An `ActorSystem` contains the states of all registered actor instances,
/// message inboxes (queues) for each registered Actor type,
/// and message dispatchers for each registered (`Actor`, `Message`) pair.
///
/// It can be controlled from the outside to do message passing and handling in turns.
pub struct ActorSystem {
    /// Flag that the system is in a panicked state
    pub panic_happened: bool,
    /// Flag that the system is shutting down
    pub shutting_down: bool,
    inboxes: [Option<Inbox>; MAX_RECIPIENT_TYPES],
    actor_registry: TypeRegistry,
    swarms: [Option<*mut u8>; MAX_RECIPIENT_TYPES],
    message_registry: TypeRegistry,
    dispatchers: [[Option<Dispatcher>; MAX_MESSAGE_TYPES]; MAX_RECIPIENT_TYPES],
    actors_as_countables: Vec<(String, *const InstancesCountable)>,
    networking: Networking,
}

macro_rules! make_array {
    ($n:expr, $constructor:expr) => {{
        let mut items: [_; $n] = ::std::mem::uninitialized();
        for (i, place) in items.iter_mut().enumerate() {
            ::std::ptr::write(place, $constructor(i));
        }
        items
    }}
}

impl ActorSystem {
    /// Create a new ActorSystem (usually only one per application is needed).
    /// Expects to get a panic callback as a parameter that is called when
    /// an actor panics during message handling and can thus be used to
    /// for example display the panic error message.
    ///
    /// Note that after an actor panicking, the whole `ActorSystem` switches
    /// to a panicked state and only passes messages anymore which have been
    /// marked as *critically receiveable* using `add_handler`.
    pub fn new(networking: Networking) -> ActorSystem {
        ActorSystem {
            panic_happened: false,
            shutting_down: false,
            inboxes: unsafe { make_array!(MAX_RECIPIENT_TYPES, |_| None) },
            actor_registry: TypeRegistry::new(),
            message_registry: TypeRegistry::new(),
            swarms: [None; MAX_RECIPIENT_TYPES],
            dispatchers: unsafe {
                make_array!(MAX_RECIPIENT_TYPES, |_| {
                    make_array!(MAX_MESSAGE_TYPES, |_| None)
                })
            },
            actors_as_countables: Vec::new(),
            networking,
        }
    }

    /// Register a new Actor type with the system
    pub fn register<A: Actor>(&mut self) {
        // allow use of actor id before it is added
        let actor_id = self.actor_registry.get_or_register::<A>();
        assert!(self.inboxes[actor_id.as_usize()].is_none());
        let actor_name = unsafe { ::std::intrinsics::type_name::<A>() };
        self.inboxes[actor_id.as_usize()] =
            Some(Inbox::new(&::chunky::Ident::from(actor_name).sub("inbox")));
        // ...but still make sure it is only added once
        assert!(self.swarms[actor_id.as_usize()].is_none());
        // Store pointer to the actor
        let actor_pointer = Box::into_raw(Box::new(Swarm::<A>::new()));
        self.swarms[actor_id.as_usize()] = Some(actor_pointer as *mut u8);
        self.actors_as_countables.push((
            self.actor_registry
                .get_name(self.actor_registry.get::<A>())
                .clone(),
            actor_pointer,
        ));
    }

    /// Register a handler for an Actor type and Message type.
    pub fn add_handler<A: Actor, M: Message, F: Fn(&M, &mut A, &mut World) -> Fate + 'static>(
        &mut self,
        handler: F,
        critical: bool,
    ) {
        let actor_id = self.actor_registry.get::<A>();
        let message_id = self.message_registry.get_or_register::<M>();
        // println!("adding to {} inbox for {}",
        //          unsafe { ::std::intrinsics::type_name::<A>() },
        //          unsafe { ::std::intrinsics::type_name::<M>() });


        let swarm_ptr = self.swarms[actor_id.as_usize()].expect("Actor not added yet") as
            *mut Swarm<A>;

        self.dispatchers[actor_id.as_usize()][message_id.as_usize()] = Some(Dispatcher {
            function: Box::new(move |packet_ptr: *const (), world: &mut World| unsafe {
                let packet = &*(packet_ptr as *const Packet<M>);

                (*swarm_ptr).dispatch_packet(packet, &handler, world);

                // TODO: not sure if this is the best place to drop the message
                ::std::ptr::drop_in_place(packet_ptr as *mut Packet<M>);
            }),
            critical: critical,
        });
    }

    /// Register a handler that constructs an instance of an Actor type, given an RawID
    pub fn add_spawner<A: Actor, M: Message, F: Fn(&M, &mut World) -> A + 'static>(
        &mut self,
        constructor: F,
        critical: bool,
    ) {
        let actor_id = self.actor_registry.get::<A>();
        let message_id = self.message_registry.get_or_register::<M>();
        // println!("adding to {} inbox for {}",
        //          unsafe { ::std::intrinsics::type_name::<A>() },
        //          unsafe { ::std::intrinsics::type_name::<M>() });


        let swarm_ptr = self.swarms[actor_id.as_usize()].expect("Actor not added yet") as
            *mut Swarm<A>;

        self.dispatchers[actor_id.as_usize()][message_id.as_usize()] = Some(Dispatcher {
            function: Box::new(move |packet_ptr: *const (), world: &mut World| unsafe {
                let packet = &*(packet_ptr as *const Packet<M>);

                let mut instance = constructor(&packet.message, world);
                (*swarm_ptr).add_manually_with_id(&mut instance, instance.id().as_raw());

                ::std::mem::forget(instance);

                // TODO: not sure if this is the best place to drop the message
                ::std::ptr::drop_in_place(packet_ptr as *mut Packet<M>);
            }),
            critical: critical,
        });
    }

    /// Send a message to the actor(s) with a given `RawID`.
    /// This is only used to send messages into the system from outside.
    /// Inside actor message handlers you always have access to a
    /// [`World`](struct.World.html) that allows you to send messages.
    pub fn send<M: Message>(&mut self, recipient: RawID, message: M) {
        let packet = Packet {
            recipient_id: recipient,
            message: message,
        };

        let to_here = recipient.machine == self.networking.machine_id;
        let global = recipient.is_global_broadcast();

        if !to_here || global {
            self.networking.enqueue(
                self.message_registry.get::<M>(),
                packet.clone(),
            );
        }

        if to_here || global {
            if let Some(inbox) = self.inboxes[recipient.type_id.as_usize()].as_mut() {
                inbox.put(packet, &self.message_registry);
            } else {
                panic!(
                    "{} has no inbox for {}",
                    self.actor_registry.get_name(recipient.type_id),
                    self.message_registry.get_name(
                        self.message_registry.get::<M>(),
                    )
                );
            }
        }
    }

    /// Get the base RawID of an Actor type
    pub fn id<A: Actor>(&mut self) -> RawID {
        RawID::new(self.short_id::<A>(), 0, self.networking.machine_id, 0)
    }

    fn short_id<A: Actor>(&mut self) -> ShortTypeId {
        self.actor_registry.get_or_register::<A>()
    }

    fn single_message_cycle(&mut self) {
        // TODO: separate inbox reading end from writing end
        //       to be able to use (several) mut refs here
        let mut world = World(self as *const Self as *mut Self);

        for (recipient_type_idx, maybe_inbox) in self.inboxes.iter_mut().enumerate() {
            if let Some(recipient_type) = ShortTypeId::new(recipient_type_idx as u16) {
                if let Some(inbox) = maybe_inbox.as_mut() {
                    for DispatchablePacket { message_type, packet_ptr } in inbox.empty() {
                        if let Some(handler) = self.dispatchers[recipient_type.as_usize()]
                            [message_type.as_usize()]
                            .as_mut()
                        {
                            if handler.critical || !self.panic_happened {
                                (handler.function)(packet_ptr, &mut world);
                            }
                        } else {
                            panic!(
                                "Dispatcher not found ({} << {})",
                                self.actor_registry.get_name(recipient_type),
                                self.message_registry.get_name(message_type)
                            );
                        }
                    }
                }
            }
        }
    }

    /// Processes all sent messages, and messages which are in turn sent
    /// during the handling of messages, up to a recursion depth of 1000.
    ///
    /// This is typically called in the main loop of an application.
    ///
    /// By sending different "top-level commands" into the system and calling
    /// `process_all_messages` inbetween, different aspects of an application
    /// (for example, UI, simulation, rendering) can be run isolated from each other,
    /// in a fixed order of "turns" during each main-loop iteration.
    pub fn process_all_messages(&mut self) {
        let result = catch_unwind(AssertUnwindSafe(|| for _i in 0..1000 {
            self.single_message_cycle();
        }));

        if result.is_err() {
            self.panic_happened = true;
        }
    }

    /// Get a world context directly from the system, typically to send messages from outside
    pub fn world(&mut self) -> World {
        World(self as *mut Self)
    }

    /// Connect to all peers in the network
    pub fn networking_connect(&mut self) {
        self.networking.connect();
    }

    /// Send queued outbound messages and take incoming queued messages
    /// and forward them to their local target recipient(s)
    pub fn networking_send_and_receive(&mut self) {
        self.networking.send_and_receive(&mut self.inboxes);
    }

    /// Finish the current networking turn and wait for peers which lag behind
    /// based on their turn number. This is the main backpressure mechanism.
    pub fn networking_finish_turn(&mut self) {
        self.networking.finish_turn(&mut self.inboxes)
    }

    /// The machine index of this machine within the network of peers
    pub fn networking_machine_id(&self) -> MachineID {
        self.networking.machine_id
    }

    /// The current network turn this machine is in. Used to keep track
    /// if this machine lags behind or runs fast compared to its peers
    pub fn networking_n_turns(&self) -> usize {
        self.networking.n_turns
    }

    /// Return a debug message containing the current local view of
    /// network turn progress of all peers in the network
    pub fn networking_debug_all_n_turns(&self) -> String {
        self.networking.debug_all_n_turns()
    }

    /// Access to debugging statistics
    pub fn get_instance_counts(&self) -> String {
        self.actors_as_countables
            .iter()
            .map(|&(ref actor_name, countable_ptr)| {
                format!(
                    "{}: {}\n", actor_name.split("::").last().unwrap().replace(">", ""),
                    unsafe {
                        (*countable_ptr).instance_count()
                    }
                )
            })
            .collect()
    }
}

/// Gives limited access to an [`ActorSystem`](struct.ActorSystem.html) (typically
/// from inside, in a message handler) to identify other actors and send messages to them.
pub struct World(*mut ActorSystem);


// TODO: make this true
unsafe impl Sync for World {}
unsafe impl Send for World {}

impl World {
    /// Send a message to a (sub-)actor with the given RawID.
    ///
    /// ```
    /// world.send(child_id, Update {dt: 1.0});
    /// ```
    pub fn send<M: Message>(&mut self, receiver: RawID, message: M) {
        unsafe { &mut *self.0 }.send(receiver, message);
    }

    /// Get the RawID of the first machine-local instance of an actor.
    pub fn local_first<A: Actor>(&mut self) -> RawID {
        unsafe { &mut *self.0 }.id::<A>()
    }

    /// Get the RawID of the first instance of an actor on machine 0
    pub fn global_first<A: Actor>(&mut self) -> RawID {
        let mut id = unsafe { &mut *self.0 }.id::<A>();
        id.machine = MachineID(0);
        id
    }

    /// Get the RawID for a broadcast to all machine-local instances of an actor.
    pub fn local_broadcast<A: Actor>(&mut self) -> RawID {
        unsafe { &mut *self.0 }.id::<A>().local_broadcast()
    }

    /// Get the RawID for a global broadcast to all instances of an actor on all machines.
    pub fn global_broadcast<A: Actor>(&mut self) -> RawID {
        unsafe { &mut *self.0 }.id::<A>().global_broadcast()
    }

    /// Synchronously allocate a instance id for a instance
    /// that will later manually be added to a Swarm
    pub fn allocate_instance_id<A: 'static + Actor>(&mut self) -> RawID {
        let system: &mut ActorSystem = unsafe { &mut *self.0 };
        let swarm = unsafe {
            &mut *(system.swarms[system.actor_registry.get::<A>().as_usize()]
                       .expect("Subactor type not found.") as *mut Swarm<A>)
        };
        unsafe { swarm.allocate_id(self.local_broadcast::<A>()) }
    }

    /// Get the id of the machine that we're currently in
    pub fn local_machine_id(&mut self) -> MachineID {
        let system: &mut ActorSystem = unsafe { &mut *self.0 };
        system.networking.machine_id
    }

    /// Signal intent to shutdown the actor system
    pub fn shutdown(&mut self) {
        let system: &mut ActorSystem = unsafe { &mut *self.0 };
        system.shutting_down = true;
    }
}

pub trait InstancesCountable {
    fn instance_count(&self) -> usize;
}

impl<T> InstancesCountable for T {
    default fn instance_count(&self) -> usize {
        1
    }
}
