use super::swarm::Swarm;
use super::messaging::{Message, Actor, Individual, Packet, Recipient};
use super::inbox::Inbox;
use super::type_registry::TypeRegistry;
use std::intrinsics::{type_id, type_name};

pub static mut THE_SYSTEM: *mut ActorSystem = 0 as *mut ActorSystem;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct ID {
    pub type_id: u16,
    pub version: u8,
    pub instance_id: u32
}

impl ID {
    pub fn invalid() -> ID {ID {type_id: u16::max_value(), version: u8::max_value(), instance_id: 0}}

    pub fn individual(individual_type_id: usize) -> ID {
        ID {type_id: individual_type_id as u16, version: 0, instance_id: 0}
    }

    pub fn broadcast(type_id: usize) -> ID {
        ID {type_id: type_id as u16, version: 0, instance_id: u32::max_value()}
    }

    pub fn instance(type_id: usize, instance_id_and_version: (usize, usize)) -> ID {
        ID {type_id: type_id as u16, version: instance_id_and_version.1 as u8, instance_id: instance_id_and_version.0 as u32}
    }

    pub fn is_broadcast(&self) -> bool {
        self.instance_id == u32::max_value()
    }

    pub fn swarm(type_id: usize) -> ID {
        ID {type_id: type_id as u16, version: 0, instance_id: u32::max_value() - 1}
    }

    pub fn is_swarm(&self) -> bool {
        self.instance_id == u32::max_value() - 1
    }
}

impl Default for ID {
    fn default() -> Self {
        ID::invalid()
    }
}

impl ::std::fmt::Debug for ID {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "ID {}_{}_{}", self.type_id, self.version, self.instance_id)
    }
}

impl<M: Message> ::std::ops::Shl<M> for ID {
    type Output = ();

    fn shl(self, rhs: M) {
        unsafe {(*THE_SYSTEM).send(self, rhs);}
    }
}

const MAX_RECIPIENT_TYPES : usize = 1024;
const MAX_MESSAGE_TYPES_PER_RECIPIENT: usize = 32;

#[derive(Clone)]
struct InboxMap {
    length: usize,
    entries: [Option<(u64, *mut u8)>; MAX_MESSAGE_TYPES_PER_RECIPIENT]
}

impl InboxMap {
    fn new() -> InboxMap {
        InboxMap{length: 0, entries: [None; MAX_MESSAGE_TYPES_PER_RECIPIENT]}
    }

    fn add_new<M: Message>(&mut self, pointer: *mut Inbox<M>) {
        let message_type_id = unsafe{type_id::<M>()};
        let entry_is_for_id = |entry: &&Option<(u64, *mut u8)>| {
            entry.is_some() && entry.unwrap().0 == message_type_id
        };
        assert!(self.entries.iter().find(entry_is_for_id).is_none());
        self.entries[self.length] = Some((message_type_id, pointer as *mut u8));
        self.length += 1;
    }

    fn get<M: Message>(&self) -> Option<*mut Inbox<M>> {
        let message_type_id = unsafe{type_id::<M>()};
        for entry in &self.entries {if let Some((id, pointer)) = *entry {
            if id == message_type_id {return Some(pointer as *mut Inbox<M>)}
        }}
        None
    }
}

pub struct ActorSystem {
    routing: [Option<InboxMap>; MAX_RECIPIENT_TYPES],
    recipient_registry: TypeRegistry,
    individuals: [Option<*mut u8>; MAX_RECIPIENT_TYPES],
    update_callbacks: Vec<Box<Fn()>>,
    clear_callbacks: Vec<Box<Fn()>>,
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
    pub fn new() -> ActorSystem {
        ActorSystem {
            routing: unsafe{make_array!(MAX_RECIPIENT_TYPES, |_| None)},
            recipient_registry: TypeRegistry::new(),
            individuals: [None; MAX_RECIPIENT_TYPES],
            update_callbacks: Vec::new(),
            clear_callbacks: Vec::new()
        }
    }

    pub fn add_individual<I: Individual>(&mut self, individual: I) {
        let recipient_id = self.recipient_registry.register_new::<I>();
        assert!(self.routing[recipient_id].is_none());
        self.routing[recipient_id] = Some(InboxMap::new());
        self.individuals[recipient_id] = Some(Box::into_raw(Box::new(individual)) as *mut u8);
    }

    pub fn add_inbox<M: Message, I: Individual + Recipient<M>> (&mut self) {
        self.add_inbox_helper::<M, I>(true);
    }

    pub fn add_unclearable_inbox<M: Message, I: Individual + Recipient<M>> (&mut self) {
        self.add_inbox_helper::<M, I>(false);
    }

    fn add_inbox_helper<M: Message, I: Individual + Recipient<M>> (&mut self, clearable: bool) {
        let inbox = Inbox::<M>::new();
        let recipient_id = self.recipient_registry.get::<I>();
        let inbox_ptr = self.store_inbox(inbox, recipient_id);
        let individual_ptr = self.individuals[recipient_id].unwrap() as *mut I;
        self.update_callbacks.push(Box::new(move || {
            unsafe {
                for packet in (*inbox_ptr).empty() {
                    (*individual_ptr).receive_packet(packet);
                }
            }
        }));
        if clearable {
            self.clear_callbacks.push(Box::new(move || {
                unsafe {
                    for _packet in (*inbox_ptr).empty() {
                    }
                }
            }));
        }
    }

    pub fn individual_id<I: Individual>(&mut self) -> ID {
        ID::individual(self.recipient_registry.get::<I>())
    }

    pub fn broadcast_id<A: Actor>(&mut self) -> ID {
        ID::broadcast(self.recipient_registry.get::<Swarm<A>>())
    }

    pub fn instance_id<A: Actor>(&mut self, instance_id_and_version: (usize, usize)) -> ID {
        ID::instance(self.recipient_registry.get::<Swarm<A>>(), instance_id_and_version)
    }

    fn store_inbox<M: Message>(&mut self, inbox: Inbox<M>, recipient_type_id: usize) -> *mut Inbox<M> {
        let inbox_ptr = Box::into_raw(Box::new(inbox));
        // TODO: deallocate inbox at the end of times
        self.routing[recipient_type_id].as_mut().unwrap().add_new(inbox_ptr);
        inbox_ptr
    }

    pub fn inbox_for<M: Message>(&mut self, packet: &Packet<M>) -> &mut Inbox<M> {
        if let Some(inbox_ptr) = self.routing[packet.recipient_id.type_id as usize].as_ref()
            .expect("Recipient not found")
            .get::<M>() {
                unsafe{&mut *inbox_ptr}
        } else {
            panic!("Inbox for {} not found for {}",
                unsafe{type_name::<M>()},
                self.recipient_registry.get_name(packet.recipient_id.type_id as usize))
        }
    }

    fn send<M: Message>(&mut self, recipient: ID, message: M) {
        let packet = Packet{
            recipient_id: recipient,
            message: message
        };
        self.inbox_for(&packet).put(packet);
    }

    pub fn process_messages(&mut self) {
        for callback in &self.update_callbacks {
            callback();
        }
    }

    pub fn clear_all_clearable_messages(&mut self) {
        for callback in &self.clear_callbacks {
            callback();
        }
    }

    pub fn process_all_messages(&mut self) {
        for _i in 0..1000 {
            self.process_messages();
        }
    }
}

impl Default for ActorSystem {
    fn default() -> Self {
        Self::new()
    }
}