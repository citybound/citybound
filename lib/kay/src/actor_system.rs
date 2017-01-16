use super::swarm::Swarm;
use super::messaging::{Message, Actor, Individual, Packet, Recipient};
use super::inbox::Inbox;
use super::id::ID;
use super::type_registry::TypeRegistry;
use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};

pub static mut THE_SYSTEM: *mut ActorSystem = 0 as *mut ActorSystem;

const MAX_RECIPIENT_TYPES: usize = 64;
const MAX_MESSAGE_TYPES: usize = 128;

struct Handler {
    function: Box<Fn(*const u8)>,
    critical: bool,
}

pub struct ActorSystem {
    panic_happened: bool,
    panic_callback: Box<Fn(Box<Any>)>,
    inboxes: [Option<Inbox>; MAX_RECIPIENT_TYPES],
    recipient_registry: TypeRegistry,
    individuals: [Option<*mut u8>; MAX_RECIPIENT_TYPES],
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
    pub fn new(panic_callback: Box<Fn(Box<Any>)>) -> ActorSystem {
        ActorSystem {
            panic_happened: false,
            panic_callback: panic_callback,
            inboxes: unsafe { make_array!(MAX_RECIPIENT_TYPES, |_| None) },
            recipient_registry: TypeRegistry::new(),
            message_registry: TypeRegistry::new(),
            individuals: [None; MAX_RECIPIENT_TYPES],
            handlers: unsafe {
                make_array!(MAX_RECIPIENT_TYPES,
                            |_| make_array!(MAX_MESSAGE_TYPES, |_| None))
            },
        }
    }

    pub fn add_individual<I: Individual>(&mut self, individual: I) {
        let recipient_id = self.recipient_registry.register_new::<I>();
        assert!(self.inboxes[recipient_id].is_none());
        self.inboxes[recipient_id] = Some(Inbox::new());
        // Store pointer to the individual
        self.individuals[recipient_id] = Some(Box::into_raw(Box::new(individual)) as *mut u8);
    }

    fn add_handler_helper<M: Message, I: Individual + Recipient<M>>(&mut self, critical: bool) {
        let recipient_id = self.recipient_registry.get::<I>();
        let message_id = self.message_registry.get_or_register::<M>();

        let individual_ptr = self.individuals[recipient_id].unwrap() as *mut I;

        self.handlers[recipient_id][message_id] = Some(Handler {
            function: Box::new(move |packet_ptr: *const u8| {
                unsafe {
                    let packet = &*(packet_ptr as *const Packet<M>);
                    (*individual_ptr).receive_packet(packet);
                }
            }),
            critical: critical,
        });
    }

    pub fn add_handler<M: Message, I: Individual + Recipient<M>>(&mut self) {
        self.add_handler_helper::<M, I>(false);
    }

    pub fn add_critical_handler<M: Message, I: Individual + Recipient<M>>(&mut self) {
        self.add_handler_helper::<M, I>(true);
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
                   self.recipient_registry.get_name(*recipient.type_id as usize));
        }
    }

    fn single_message_cycle(&mut self) {
        for (recipient_type, maybe_inbox) in self.inboxes.iter_mut().enumerate() {
            if let Some(inbox) = maybe_inbox.as_mut() {
                for (message_type, ptr) in inbox.empty() {
                    if let Some(handler) = self.handlers[recipient_type][message_type].as_mut() {
                        if handler.critical || !self.panic_happened {
                            (handler.function)(ptr);
                        }
                    } else {
                        panic!("Handler not found ({} << {})",
                               self.recipient_registry.get_name(recipient_type),
                               self.message_registry.get_name(message_type));
                    }
                }
            }
        }
    }

    pub fn process_all_messages(&mut self) {
        let result = catch_unwind(AssertUnwindSafe(|| {
            for _i in 0..1000 {
                self.single_message_cycle();
            }
        }));

        if result.is_err() {
            self.panic_happened = true;
            (self.panic_callback)(result.unwrap_err());
        }
    }

    pub fn individual_id<I: Individual>(&mut self) -> ID {
        ID::individual(self.recipient_registry.get::<I>())
    }

    pub fn broadcast_id<A: Actor>(&mut self) -> ID {
        ID::broadcast(self.recipient_registry.get::<Swarm<A>>())
    }

    pub fn instance_id<A: Actor>(&mut self, instance_id_and_version: (usize, usize)) -> ID {
        ID::instance(self.recipient_registry.get::<Swarm<A>>(),
                     instance_id_and_version)
    }
}