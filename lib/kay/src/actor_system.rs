use super::messaging::{Message, Packet, Recipient};
use super::inbox::{Inbox, DispatchablePacket};
use super::id::ID;
use super::type_registry::{ShortTypeId, TypeRegistry};
use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};

pub static mut THE_SYSTEM: *mut ActorSystem = 0 as *mut ActorSystem;

const MAX_RECIPIENT_TYPES: usize = 64;
const MAX_MESSAGE_TYPES: usize = 128;

struct Handler {
    function: Box<Fn(*const ())>,
    critical: bool,
}

pub trait Actor: 'static + Sized {
    fn register_with_state(initial_state: Self) {
        unsafe { (*THE_SYSTEM).add_actor(initial_state) };
    }

    fn register_default()
        where Self: Default
    {
        Self::register_with_state(Self::default());
    }

    fn id() -> ID {
        ID::new(unsafe { (*THE_SYSTEM).short_id::<Self>() }, 0, 0)
    }

    fn handle<M: Message>()
        where Self: Recipient<M>
    {
        unsafe { (*THE_SYSTEM).add_handler::<M, Self>() }
    }

    fn handle_critically<M: Message>()
        where Self: Recipient<M>
    {
        unsafe { (*THE_SYSTEM).add_critical_handler::<M, Self>() }
    }
}

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

    pub fn process_all_messages(&mut self) {
        let result = catch_unwind(AssertUnwindSafe(|| for _i in 0..1000 {
            self.single_message_cycle();
        }));

        if result.is_err() {
            self.panic_happened = true;
            (self.panic_callback)(result.unwrap_err());
        }
    }

    pub fn short_id<A: Actor>(&self) -> ShortTypeId {
        self.actor_registry.get::<A>()
    }
}
