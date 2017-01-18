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

pub trait Individual: 'static + Sized {
    fn id() -> ID {
        ID::new(unsafe { (*super::THE_SYSTEM).short_id::<Self>() }, 0, 0)
    }

    fn handle<M: Message>()
        where Self: Recipient<M>
    {
        unsafe { (*super::THE_SYSTEM).add_handler::<M, Self>() }
    }

    fn handle_critically<M: Message>()
        where Self: Recipient<M>
    {
        unsafe { (*super::THE_SYSTEM).add_critical_handler::<M, Self>() }
    }
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
        assert!(self.inboxes[recipient_id.as_usize()].is_none());
        self.inboxes[recipient_id.as_usize()] = Some(Inbox::new());
        // Store pointer to the individual
        self.individuals[recipient_id.as_usize()] = Some(Box::into_raw(Box::new(individual)) as
                                                         *mut u8);
    }

    fn add_handler_helper<M: Message, I: Individual + Recipient<M>>(&mut self, critical: bool) {
        let recipient_id = self.recipient_registry.get::<I>();
        let message_id = self.message_registry.get_or_register::<M>();

        let individual_ptr = self.individuals[recipient_id.as_usize()].unwrap() as *mut I;

        self.handlers[recipient_id.as_usize()][message_id.as_usize()] = Some(Handler {
            function: Box::new(move |packet_ptr: *const ()| unsafe {
                let packet = &*(packet_ptr as *const Packet<M>);
                (*individual_ptr).receive_packet(packet);
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
                   self.recipient_registry.get_name(recipient.type_id));
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
                               self.recipient_registry.get_name(recipient_type),
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

    pub fn short_id<I: Individual>(&self) -> ShortTypeId {
        self.recipient_registry.get::<I>()
    }
}
