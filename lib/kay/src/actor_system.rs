use super::messaging::{Message, Packet, Fate};
use super::inbox::{Inbox, DispatchablePacket};
use super::id::ID;
use super::type_registry::{ShortTypeId, TypeRegistry};
use super::swarm::{Swarm, SubActor};
use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};

struct Dispatcher {
    function: Box<Fn(*const (), &mut World)>,
    critical: bool,
}

const MAX_RECIPIENT_TYPES: usize = 64;
const MAX_MESSAGE_TYPES: usize = 128;

/// The main thing inside of which all the magic happens.
///
/// An `ActorSystem` contains the states of all registered actors,
/// message inboxes (queues) for each registered actor,
/// and message dispatchers for each registered (`Actor`,`Message`) pair.
///
/// It can be controlled from the outside to do message passing and handling in turns.
pub struct ActorSystem {
    panic_happened: bool,
    panic_callback: Box<Fn(Box<Any>, &mut World)>,
    inboxes: [Option<Inbox>; MAX_RECIPIENT_TYPES],
    actor_registry: TypeRegistry,
    actors: [Option<*mut u8>; MAX_RECIPIENT_TYPES],
    message_registry: TypeRegistry,
    dispatchers: [[Option<Dispatcher>; MAX_MESSAGE_TYPES]; MAX_RECIPIENT_TYPES],
    actors_as_countables: Vec<(String, *const SubActorsCountable)>,
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
    /// marked as *critically received* using `ActorDefiner::on_critical`.
    pub fn new(panic_callback: Box<Fn(Box<Any>, &mut World)>) -> ActorSystem {
        ActorSystem {
            panic_happened: false,
            panic_callback: panic_callback,
            inboxes: unsafe { make_array!(MAX_RECIPIENT_TYPES, |_| None) },
            actor_registry: TypeRegistry::new(),
            message_registry: TypeRegistry::new(),
            actors: [None; MAX_RECIPIENT_TYPES],
            dispatchers: unsafe {
                make_array!(MAX_RECIPIENT_TYPES, |_| {
                    make_array!(MAX_MESSAGE_TYPES, |_| None)
                })
            },
            actors_as_countables: Vec::new(),
        }
    }

    /// Add a new actor to the system given an initial actor state
    /// and a closure that takes an [ActorDefiner](struct.ActorDefiner.html)
    /// to define message handlers for this actor.
    ///
    /// ```
    /// system.add(Logger::new(), |mut the_logger| {
    ///     the_logger.on(|&Message { text }, logger, world| {
    ///         //...
    ///     });
    /// });
    /// ```
    pub fn add<A: 'static, D: Fn(ActorDefiner<A>)>(&mut self, actor: A, define: D) {
        // allow use of actor id before it is added
        let actor_id = self.actor_registry.get_or_register::<A>();
        assert!(self.inboxes[actor_id.as_usize()].is_none());
        self.inboxes[actor_id.as_usize()] = Some(Inbox::new());
        // ...but still make sure it is only added once
        assert!(self.actors[actor_id.as_usize()].is_none());
        // Store pointer to the actor
        let actor_pointer = Box::into_raw(Box::new(actor));
        self.actors[actor_id.as_usize()] = Some(actor_pointer as *mut u8);
        self.actors_as_countables.push((
            self.actor_registry
                .get_name(self.actor_registry.get::<A>())
                .clone(),
            actor_pointer,
        ));
        define(ActorDefiner::with(self));
    }

    /// Extend a previously added actor using a closure that
    /// takes an [ActorDefiner](struct.ActorDefiner.html)
    /// to define *additional* message handlers for this actor.
    ///
    /// This is useful if you want to implement different *aspects*
    /// of an actor in different files.
    ///
    /// ```
    /// system.extend::<Logger, _>(|mut the_logger| {
    ///     the_logger.on(|_: &ClearAll, logger, world| {
    ///         //...
    ///     });
    /// });
    /// ```
    pub fn extend<A: 'static, D: Fn(ActorDefiner<A>)>(&mut self, define: D) {
        define(ActorDefiner::with(self));
    }

    fn add_dispatcher<
        M: Message,
        A: 'static,
        F: Fn(&Packet<M>, &mut A, &mut World) -> Fate + 'static,
    >(
        &mut self,
        handler: F,
        critical: bool,
    ) {
        let actor_id = self.actor_registry.get::<A>();
        let message_id = self.message_registry.get_or_register::<M>();
        // println!("adding to {} inbox for {}",
        //          unsafe { ::std::intrinsics::type_name::<A>() },
        //          unsafe { ::std::intrinsics::type_name::<M>() });


        let actor_ptr = self.actors[actor_id.as_usize()].unwrap() as *mut A;

        self.dispatchers[actor_id.as_usize()][message_id.as_usize()] = Some(Dispatcher {
            function: Box::new(move |packet_ptr: *const (), world: &mut World| unsafe {
                let packet = &*(packet_ptr as *const Packet<M>);
                handler(packet, &mut *actor_ptr, world);
                // TODO: not sure if this is the best place to drop the message
                ::std::ptr::drop_in_place(packet_ptr as *mut Packet<M>);
            }),
            critical: critical,
        });
    }

    /// Send a message to the actor with a given `ID`.
    /// This is only used to send messages into the system from outside.
    /// Inside actor message handlers you always have access to a
    /// [`World`](struct.World.html) that allows you to send messages.
    pub fn send<M: Message>(&mut self, recipient: ID, message: M) {
        let packet = Packet {
            recipient_id: Some(recipient),
            message: message,
        };

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

    /// Get the ID of a previously added actor.
    /// This is only used to identify actors from the outside.
    /// Inside actor message handlers you always have access to a
    /// [`World`](struct.World.html) that allows you to identify actors.
    pub fn id<A2: 'static>(&mut self) -> ID {
        ID::new(self.short_id::<A2>(), 0, 0)
    }

    fn short_id<A: 'static>(&mut self) -> ShortTypeId {
        self.actor_registry.get_or_register::<A>()
    }

    fn single_message_cycle(&mut self) {
        // TODO: separate inbox reading end from writing end
        //       to be able to use (several) mut refs here
        let mut world = World(self as *const Self as *mut Self);

        for (recipient_type_idx, maybe_inbox) in self.inboxes.iter_mut().enumerate() {
            let recipient_type = ShortTypeId::new(recipient_type_idx as u16);
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

    /// Processes all sent messages, and messages which are in turn sent
    /// during the handling of messages, up to a recursion depth of 1000.
    ///
    /// This is typically called in the main loop of an application.
    ///
    /// By sending different "top-level commands" in to the system and calling
    /// `process_all_messages` inbetween, different aspects of an application
    /// (for example, UI, simulation, rendering) can be run isolated from each other,
    /// in a fixed order of "turns" during each main-loop iteration.
    pub fn process_all_messages(&mut self) {
        let result = catch_unwind(AssertUnwindSafe(|| for _i in 0..1000 {
            self.single_message_cycle();
        }));

        if result.is_err() {
            self.panic_happened = true;
            (self.panic_callback)(
                result.unwrap_err(),
                &mut World(self as *const Self as *mut Self),
            );
        }
    }

    /// Get a world context directly from the system, typically to send messages from outside
    pub fn world(&mut self) -> World {
        World(self as *mut Self)
    }

    /// Access to debugging statistics
    pub fn get_subactor_counts(&self) -> String {
        self.actors_as_countables
            .iter()
            .map(|&(ref actor_name, countable_ptr)| {
                format!("{}: {}\n", actor_name.split("::").last().unwrap().replace(">", ""), unsafe {
                    (*countable_ptr).subactor_count()
                })
            })
            .collect()
    }
}

/// Helper that is used to define actor behaviour (message handlers).
///
/// It is passed to the closure arguments of
/// [`ActorSystem::add`](struct.ActorSystem.html#method.add) and
/// [`ActorSystem::extend`](struct.ActorSystem.html#method.extend)
pub struct ActorDefiner<'a, A> {
    system: &'a mut ActorSystem,
    marker: ::std::marker::PhantomData<A>,
}

impl<'a, A: 'static> ActorDefiner<'a, A> {
    fn with(system: &'a mut ActorSystem) -> Self {
        ActorDefiner {
            system: system,
            marker: ::std::marker::PhantomData,
        }
    }

    /// Attach a new message handler to an actor, defined by a closure
    /// which will receive 3 arguments:
    ///
    /// * the received message (can conveniently be destructured already as an argument)
    /// * the current actor state that can be mutated
    /// * a [`World`](struct.World.html) to identify and send
    ///   messages to other actors
    ///
    /// ```
    /// the_counter.on(|&IncrementCount { increment }, counter, world| {
    ///     counter.count += increment;
    ///     world.send_to_id_of::<Logger, _>(format!("New count: {}", counter.count));
    /// });
    /// ```
    pub fn on<M: Message, F>(&mut self, handler: F)
    where
        F: Fn(&M, &mut A, &mut World) -> Fate + 'static,
    {
        self.on_maybe_critical(handler, false);
    }

    /// Same as [`on`](#method.on) but continues to receive after the ActorSystem panicked.
    pub fn on_critical<M: Message, F>(&mut self, handler: F)
    where
        F: Fn(&M, &mut A, &mut World) -> Fate + 'static,
    {
        self.on_maybe_critical(handler, true);
    }

    /// Access a [`World`](struct.World.html) of the system that an actor
    /// is being defined in, can be used to identify actors (and keep the ID)
    /// or send messages at *define-time*.
    ///
    /// ```
    /// let logger_id = the_counter.world().id::<Logger>();
    ///
    /// the_counter.on(move |&IncrementCount { increment }, counter, world| {
    ///     counter.count += increment;
    ///     world.send(logger_id, format!("New count: {}", counter.count));
    /// });
    ///
    /// the_counter.world().send(logger_id, "Just defined Counter!");
    /// ```
    pub fn world(&mut self) -> World {
        World(self.system as *mut ActorSystem)
    }

    /// Advanced: Can be used to register a handler not only for a message,
    /// but for a whole packet (precise recipient id + message), in case
    /// a particular sub-actor needs to be identified in the handler,
    /// like [`Swarm`](swarm/struct.Swarm.html) does.
    pub fn on_packet<M: Message, F>(&mut self, handler: F, critical: bool)
    where
        F: Fn(&Packet<M>, &mut A, &mut World) -> Fate + 'static,
    {
        self.system.add_dispatcher(handler, critical);
    }

    fn on_maybe_critical<M: Message, F>(&mut self, handler: F, critical: bool)
    where
        F: Fn(&M, &mut A, &mut World) -> Fate + 'static,
    {
        self.system.add_dispatcher(
            move |packet: &Packet<M>, state, world| handler(&packet.message, state, world),
            critical,
        );
    }
}

/// Gives limited access to an [`ActorSystem`](struct.ActorSystem.html) (typically
/// from inside, in a message handler) to identify other actors and send messages to them.
pub struct World(*mut ActorSystem);

impl World {
    /// Send a message to a (sub-)actor with the given ID.
    ///
    /// ```
    /// world.send(child_id, Update {dt: 1.0});
    /// ```
    pub fn send<M: Message>(&mut self, receiver: ID, message: M) {
        unsafe { &mut *self.0 }.send(receiver, message);
    }

    /// Identify an actor based on type.
    ///
    /// ```
    /// let logger_id = world.id::<Logger>();
    /// ```
    pub fn id<A2: 'static>(&mut self) -> ID {
        unsafe { &mut *self.0 }.id::<A2>()
    }

    /// Shorthand for identifying an actor and then sending a message to it.
    ///
    /// ```
    /// world.send_to_id_of::<Logger, _>("New message!");
    /// // is equivalent to
    /// let logger_id = world.id::<Logger>();
    /// world.send(logger_id, "New message!");
    /// ```
    pub fn send_to_id_of<A: 'static, M: Message>(&mut self, message: M) {
        let id = self.id::<A>();
        self.send(id, message);
    }

    /// Shorthand to broadcast something to all subactors of a [`Swarm`](swarm/struct.Swarm.html).
    ///
    /// ```
    /// world.broadcast_to_id_of::<UIElement, _>(UpdateUI);
    /// // is equivalent to
    /// let all_elements = world.id::<Swarm<UIElement>>().broadcast();
    /// world.send(all_elements, UpdateUI);
    /// ```
    pub fn broadcast_to_id_of<A: 'static, M: Message>(&mut self, message: M) {
        let id = self.id::<A>().broadcast();
        self.send(id, message);
    }

    /// Synchronously allocate a subactor id for a subactor
    /// that will later manually be added to a Swarm
    pub fn allocate_subactor_id<SA: 'static + SubActor>(&mut self) -> ID {
        let system: &mut ActorSystem = unsafe { &mut *self.0 };
        let swarm = unsafe {
            &mut *(system.actors[system.actor_registry.get::<Swarm<SA>>().as_usize()]
                       .expect("Subactor type not found.") as *mut Swarm<SA>)
        };
        unsafe { swarm.allocate_id(self.id::<Swarm<SA>>()) }
    }
}

pub trait SubActorsCountable {
    fn subactor_count(&self) -> usize;
}

impl<T> SubActorsCountable for T {
    default fn subactor_count(&self) -> usize {
        1
    }
}
