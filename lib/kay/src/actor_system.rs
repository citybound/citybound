use super::messaging::{Message, Packet, Recipient};
use super::inbox::{Inbox, DispatchablePacket};
use super::id::ID;
use super::type_registry::{ShortTypeId, TypeRegistry};
use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};

/// Global pointer to the singleton instance of `ActorSystem`.
/// Set by `ActorSystem::create_the_system`.
pub static mut THE_SYSTEM: *mut ActorSystem = 0 as *mut ActorSystem;

const MAX_RECIPIENT_TYPES: usize = 64;
const MAX_MESSAGE_TYPES: usize = 128;

struct Handler {
    function: Box<Fn(*const ())>,
    critical: bool,
}

/// Any Rust data structure can become an actor by:
///
///  * Deriving `Actor`
///  * Deriving `Recipient<M>` for all message types that this actor will handle
///  * Registering itself and all handled messages with the actor system
pub trait Actor: 'static + Sized {
    /// Register an `Actor` with the system, using the given instance
    /// of the actor type as the initial state for the actor.
    ///
    /// After this call, the Actor has an ID (see `id()`) and can receive messages
    /// (if according message handlers have been registered with `handle` or `handle_critically`)
    fn register_with_state(initial_state: Self) {
        unsafe { (*THE_SYSTEM).add_actor(initial_state) };
    }

    /// Like `register_with_state`, using the default value for `Actor`s that implement `Default`
    fn register_default()
        where Self: Default
    {
        Self::register_with_state(Self::default());
    }

    /// Get the `ID` of an `Actor` (only works after the actor has been registered)
    fn id() -> ID {
        ID::new(unsafe { (*THE_SYSTEM).short_id::<Self>() }, 0, 0)
    }

    /// Register a message handler for a message `M` for an `Actor`,
    /// which is defined in the implementation of `Recipient<M> for this actor.
    fn handle<M: Message>()
        where Self: Recipient<M>
    {
        unsafe { (*THE_SYSTEM).add_handler::<M, Self>() }
    }

    /// Like `handle`, but marks this message type as *critical* for `Actor`,
    /// which means that this actor will still receive such messages even
    /// after a panic occured in the `ActorSystem`
    ///
    /// This makes it possible to keep for example rendering, debug message display
    /// and camera movement alive, even after an otherwise fatal panic.
    fn handle_critically<M: Message>()
        where Self: Recipient<M>
    {
        unsafe { (*THE_SYSTEM).add_critical_handler::<M, Self>() }
    }
}

/// An `ActorSystem` contains the states of all registered actors,
/// message inboxes (queues) for each registered actor,
/// and message handlers for each registered (`Actor`,`Message`) pair.
pub struct ActorSystem {
    panic_happened: bool,
    panic_callback: Box<Fn(Box<Any>)>,
    inboxes: [Option<Inbox>; MAX_RECIPIENT_TYPES],
    actor_registry: TypeRegistry,
    actors: [Option<*mut u8>; MAX_RECIPIENT_TYPES],
    message_registry: TypeRegistry,
    handlers: [[Option<Handler>; MAX_MESSAGE_TYPES]; MAX_RECIPIENT_TYPES],
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
    /// Create the global singleton instance of `ActorSystem`.
    /// Sets `THE_SYSTEM`. Expects to get a panic callback as a parameter
    /// that is called when an actor panics during message handling
    /// and can thus be used to for example display the panic error message.
    ///
    /// Note that after an actor panicking, the whole `ActorSystem` switches
    /// to a panicked state and only passes messages anymore which have been
    /// marked as *critical* using `Actor::handle_critically`.
    pub fn create_the_system(panic_callback: Box<Fn(Box<Any>)>) -> Box<ActorSystem> {
        let mut system = Box::new(ActorSystem {
            panic_happened: false,
            panic_callback: panic_callback,
            inboxes: unsafe { make_array!(MAX_RECIPIENT_TYPES, |_| None) },
            actor_registry: TypeRegistry::new(),
            message_registry: TypeRegistry::new(),
            actors: [None; MAX_RECIPIENT_TYPES],
            handlers: unsafe {
                make_array!(MAX_RECIPIENT_TYPES,
                            |_| make_array!(MAX_MESSAGE_TYPES, |_| None))
            },
        });

        unsafe {
            THE_SYSTEM = &mut *system as *mut ActorSystem;
        }

        system
    }

    fn add_actor<A: Actor>(&mut self, actor: A) {
        let actor_id = self.actor_registry.register_new::<A>();
        assert!(self.inboxes[actor_id.as_usize()].is_none());
        self.inboxes[actor_id.as_usize()] = Some(Inbox::new());
        // Store pointer to the actor
        self.actors[actor_id.as_usize()] = Some(Box::into_raw(Box::new(actor)) as *mut u8);
    }

    fn add_handler_helper<M: Message, A: Actor + Recipient<M>>(&mut self, critical: bool) {
        let actor_id = self.actor_registry.get::<A>();
        let message_id = self.message_registry.get_or_register::<M>();

        let actor_ptr = self.actors[actor_id.as_usize()].unwrap() as *mut A;

        self.handlers[actor_id.as_usize()][message_id.as_usize()] = Some(Handler {
            function: Box::new(move |packet_ptr: *const ()| unsafe {
                let packet = &*(packet_ptr as *const Packet<M>);
                (*actor_ptr).receive_packet(packet);
            }),
            critical: critical,
        });
    }

    fn add_handler<M: Message, A: Actor + Recipient<M>>(&mut self) {
        self.add_handler_helper::<M, A>(false);
    }

    fn add_critical_handler<M: Message, A: Actor + Recipient<M>>(&mut self) {
        self.add_handler_helper::<M, A>(true);
    }

    /// Send a message to the actor with a given `ID`. This is usually never called directly,
    /// instead, [`ID << Message`](struct.ID.html#method.shl) is used to send messages.
    pub fn send<M: Message>(&mut self, recipient: ID, message: M) {
        let packet = Packet {
            recipient_id: Some(recipient),
            message: message,
        };

        if let Some(inbox) = self.inboxes[*recipient.type_id as usize].as_mut() {
            inbox.put(packet, &self.message_registry);
        } else {
            panic!("No inbox for {}",
                   self.actor_registry.get_name(recipient.type_id));
        }
    }

    fn single_message_cycle(&mut self) {
        for (recipient_type_idx, maybe_inbox) in self.inboxes.iter_mut().enumerate() {
            let recipient_type = ShortTypeId::new(recipient_type_idx as u16);
            if let Some(inbox) = maybe_inbox.as_mut() {
                for DispatchablePacket { message_type, packet_ptr } in inbox.empty() {
                    if let Some(handler) = self.handlers[recipient_type.as_usize()]
                                               [message_type.as_usize()]
                        .as_mut() {
                        if handler.critical || !self.panic_happened {
                            (handler.function)(packet_ptr);
                        }
                    } else {
                        panic!("Handler not found ({} << {})",
                               self.actor_registry.get_name(recipient_type),
                               self.message_registry.get_name(message_type));
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
    /// in a fixed order during each loop iteration.
    pub fn process_all_messages(&mut self) {
        let result = catch_unwind(AssertUnwindSafe(|| for _i in 0..1000 {
            self.single_message_cycle();
        }));

        if result.is_err() {
            self.panic_happened = true;
            (self.panic_callback)(result.unwrap_err());
        }
    }

    /// Get the short type id for an `Actor`. Ususally never called directly,
    /// use `Actor::id()` instead to get the full ID of a registered Actor.
    pub fn short_id<A: Actor>(&self) -> ShortTypeId {
        self.actor_registry.get::<A>()
    }
}
