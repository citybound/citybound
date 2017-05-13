use super::messaging::{Message, Packet, Fate};
use super::inbox::{Inbox, DispatchablePacket};
use super::id::ID;
use super::type_registry::{ShortTypeId, TypeRegistry};
use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};

struct Dispatcher {
    function: Box<Fn(*const (), &mut World)>,
    critical: bool,
}

const MAX_RECIPIENT_TYPES: usize = 64;
const MAX_MESSAGE_TYPES: usize = 128;

/// An `ActorSystem` contains the states of all registered actors,
/// message inboxes (queues) for each registered actor,
/// and message dispatchers for each registered (`Actor`,`Message`) pair.
pub struct ActorSystem {
    panic_happened: bool,
    panic_callback: Box<Fn(Box<Any>, &mut World)>,
    inboxes: [Option<Inbox>; MAX_RECIPIENT_TYPES],
    actor_registry: TypeRegistry,
    actors: [Option<*mut u8>; MAX_RECIPIENT_TYPES],
    message_registry: TypeRegistry,
    dispatchers: [[Option<Dispatcher>; MAX_MESSAGE_TYPES]; MAX_RECIPIENT_TYPES],
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
    pub fn new(panic_callback: Box<Fn(Box<Any>, &mut World)>) -> ActorSystem {
        ActorSystem {
            panic_happened: false,
            panic_callback: panic_callback,
            inboxes: unsafe { make_array!(MAX_RECIPIENT_TYPES, |_| None) },
            actor_registry: TypeRegistry::new(),
            message_registry: TypeRegistry::new(),
            actors: [None; MAX_RECIPIENT_TYPES],
            dispatchers: unsafe {
                make_array!(MAX_RECIPIENT_TYPES,
                            |_| make_array!(MAX_MESSAGE_TYPES, |_| None))
            },
        }
    }

    pub fn add<A: 'static, D: Fn(ActorDefiner<A>)>(&mut self, actor: A, define: D) {
        // allow use of actor id before it is added
        let actor_id = self.actor_registry.get_or_register::<A>();
        assert!(self.inboxes[actor_id.as_usize()].is_none());
        self.inboxes[actor_id.as_usize()] = Some(Inbox::new());
        // ...but still make sure it is only added once
        assert!(self.actors[actor_id.as_usize()].is_none());
        // Store pointer to the actor
        self.actors[actor_id.as_usize()] = Some(Box::into_raw(Box::new(actor)) as *mut u8);
        define(ActorDefiner::with(self));
    }

    pub fn extend<A: 'static, D: Fn(ActorDefiner<A>)>(&mut self, define: D) {
        define(ActorDefiner::with(self));
    }

    fn add_dispatcher<M: Message,
                      A: 'static,
                      F: Fn(&Packet<M>, &mut A, &mut World) -> Fate + 'static>
        (&mut self,
         handler: F,
         critical: bool) {
        let actor_id = self.actor_registry.get::<A>();
        let message_id = self.message_registry.get_or_register::<M>();
        // println!("adding to {} inbox for {}",
        //          unsafe { ::std::intrinsics::type_name::<A>() },
        //          unsafe { ::std::intrinsics::type_name::<M>() });


        let actor_ptr = self.actors[actor_id.as_usize()].unwrap() as *mut A;

        self.dispatchers[actor_id.as_usize()][message_id.as_usize()] =
            Some(Dispatcher {
                     function: Box::new(move |packet_ptr: *const (), world: &mut World| unsafe {
                let packet = &*(packet_ptr as *const Packet<M>);
                handler(packet, &mut *actor_ptr, world);
            }),
                     critical: critical,
                 });
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
            panic!("{} has no inbox for {}",
                   self.actor_registry.get_name(recipient.type_id),
                   self.message_registry
                       .get_name(self.message_registry.get::<M>()));
        }
    }

    pub fn id<A2: 'static>(&mut self) -> ID {
        ID::new(self.short_id::<A2>(), 0, 0)
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
                               .as_mut() {
                        if handler.critical || !self.panic_happened {
                            (handler.function)(packet_ptr, &mut world);
                        }
                    } else {
                        panic!("Dispatcher not found ({} << {})",
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
            (self.panic_callback)(result.unwrap_err(),
                                  &mut World(self as *const Self as *mut Self));
        }
    }

    /// Get the short type id for an `Actor`. Ususally never called directly,
    /// use `Actor::id()` instead to get the full ID of a registered Actor.
    pub fn short_id<A: 'static>(&mut self) -> ShortTypeId {
        self.actor_registry.get_or_register::<A>()
    }
}

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

    pub fn on_packet<M: Message, F>(&mut self, handler: F, critical: bool)
        where F: Fn(&Packet<M>, &mut A, &mut World) -> Fate + 'static
    {
        self.system.add_dispatcher(handler, critical);
    }

    pub fn on_maybe_critical<M: Message, F>(&mut self, handler: F, critical: bool)
        where F: Fn(&M, &mut A, &mut World) -> Fate + 'static
    {
        self.system
            .add_dispatcher(move |packet: &Packet<M>, state, world| {
                handler(&packet.message, state, world)
            },
                            critical);
    }

    pub fn on<M: Message, F>(&mut self, handler: F)
        where F: Fn(&M, &mut A, &mut World) -> Fate + 'static
    {
        self.on_maybe_critical(handler, false);
    }

    pub fn on_critical<M: Message, F>(&mut self, handler: F)
        where F: Fn(&M, &mut A, &mut World) -> Fate + 'static
    {
        self.on_maybe_critical(handler, true);
    }

    pub fn world(&mut self) -> World {
        World(self.system as *mut ActorSystem)
    }
}

pub struct World(*mut ActorSystem);

impl World {
    pub fn send<M: Message>(&mut self, receiver: ID, message: M) {
        unsafe { &mut *self.0 }.send(receiver, message);
    }

    pub fn id<A2: 'static>(&mut self) -> ID {
        unsafe { &mut *self.0 }.id::<A2>()
    }

    pub fn send_to_id_of<A: 'static, M: Message>(&mut self, message: M) {
        let id = self.id::<A>();
        self.send(id, message);
    }

    pub fn broadcast_to_id_of<A: 'static, M: Message>(&mut self, message: M) {
        let id = self.id::<A>().broadcast();
        self.send(id, message);
    }
}
