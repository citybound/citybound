use super::swarm::Swarm;
use super::messaging::{Message, Actor, Individual, Packet, Recipient};
use super::inbox::Inbox;
use super::type_registry::TypeRegistry;
use std::intrinsics::{type_id, type_name};

/// The single, global actor system for the whole program
pub static mut THE_SYSTEM: *mut ActorSystem = 0 as *mut ActorSystem;

/// The ID used to specify a specific object within the actor system
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct ID {
    /// The sequentially numbered type ID of the actor within the actor system
    pub type_id: u16,
    /// The version of the game which the actor was created in
    pub version: u8,
    /// The instance of the type used to address a specific actor
    /// Is broadcast if equal to `u32::max_value()1`
    /// Is swarm if equal to `u32::max_value() -1`
    pub instance_id: u32
}

impl ID {
    /// Construct an invalid ID
    /// Uses similarly to a null pointer
    pub fn invalid() -> ID {ID {type_id: u16::max_value(), version: u8::max_value(), instance_id: 0}}

    /// Construct an individual actor ID with instance ID of 0 and the typeID specified
    pub fn individual(individual_type_id: usize) -> ID {
        ID {type_id: individual_type_id as u16, version: 0, instance_id: 0}
    }

    /// Construct a broadcast ID to all actors with the type ID specified
    pub fn broadcast(type_id: usize) -> ID {
        ID {type_id: type_id as u16, version: 0, instance_id: u32::max_value()}
    }

    /// Construct an individual actor ID
    pub fn instance(type_id: usize, instance_id_and_version: (usize, usize)) -> ID {
        ID {type_id: type_id as u16, version: instance_id_and_version.1 as u8, instance_id: instance_id_and_version.0 as u32}
    }

    /// Checks if ID is a broadcast ID
    pub fn is_broadcast(&self) -> bool {
        self.instance_id == u32::max_value()
    }

    /// Created swarm ID with type ID specified
    pub fn swarm(type_id: usize) -> ID {
        ID {type_id: type_id as u16, version: 0, instance_id: u32::max_value() - 1}
    }

    /// Checks if ID is a swarm ID
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
    /// The shift left operator is overloaded to use to send messages
    /// e.g. recipient_ID << message
    fn shl(self, rhs: M) {
        unsafe {(*THE_SYSTEM).send(self, rhs);}
    }
}

const MAX_RECIPIENT_TYPES : usize = 64;
const MAX_MESSAGE_TYPES_PER_RECIPIENT: usize = 32;

#[derive(Clone)]
struct InboxMap {
    /// Amount of valid entries in the InboxMap
    length: usize,
    /// Entry consists of TypeID of Message type and pointer to inbox
    entries: [Option<(u64, *mut u8)>; MAX_MESSAGE_TYPES_PER_RECIPIENT]
}

impl InboxMap {
    /// Creates an empty InboxMap
    fn new() -> InboxMap {
        InboxMap{length: 0, entries: [None; MAX_MESSAGE_TYPES_PER_RECIPIENT]}
    }

    /// Adds new message type to the InboxMap
    fn add_new<M: Message>(&mut self, pointer: *mut Inbox<M>) {
        let message_type_id = unsafe{type_id::<M>()};
        let entry_is_for_id = |entry: &&Option<(u64, *mut u8)>| {
            entry.is_some() && entry.unwrap().0 == message_type_id
        };
        assert!(self.entries.iter().find(entry_is_for_id).is_none());
        self.entries[self.length] = Some((message_type_id, pointer as *mut u8));
        self.length += 1;
    }

    /// Gets the inbox pointer of the type specified if there is one
    fn get<M: Message>(&self) -> Option<*mut Inbox<M>> {
        let message_type_id = unsafe{type_id::<M>()};
        for entry in &self.entries {if let Some((id, pointer)) = *entry {
            if id == message_type_id {return Some(pointer as *mut Inbox<M>)}
        }}
        None
    }
}

pub struct ActorSystem {
    /// Stores the inboxes of all the swarms
    routing: [Option<InboxMap>; MAX_RECIPIENT_TYPES],
    /// Stores the rust TypeID to internal ID and type name mapping
    recipient_registry: TypeRegistry,
    /// Stores all swarms
    individuals: [Option<*mut u8>; MAX_RECIPIENT_TYPES],
    /// Closures to process and clear messages
    update_callbacks: Vec<Box<Fn()>>,
    /// Closures to clear messages
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
    /// Creates new actor system
    pub fn new() -> ActorSystem {
        ActorSystem {
            routing: unsafe{make_array!(MAX_RECIPIENT_TYPES, |_| None)},
            recipient_registry: TypeRegistry::new(),
            individuals: [None; MAX_RECIPIENT_TYPES],
            update_callbacks: Vec::new(),
            clear_callbacks: Vec::new()
        }
    }

    /// Registers a type for use as an actor in the actor system
    pub fn add_individual<I: Individual>(&mut self, individual: I) {
        // Register type in recipient_registry, and return the short ID of the type (sequential ID starting from 0)
        let recipient_id = self.recipient_registry.register_new::<I>();

        // Check inbox does not exist at routing[recipient_id]
        assert!(self.routing[recipient_id].is_none());

        self.routing[recipient_id] = Some(InboxMap::new());

        // Store pointer to the Swarm
        self.individuals[recipient_id] = Some(Box::into_raw(Box::new(individual)) as *mut u8);
    }

    /// Add a inbox for a given message type to a individual
    pub fn add_inbox<M: Message, I: Individual + Recipient<M>> (&mut self) {
        self.add_inbox_helper::<M, I>(true);
    }

    /// Add an unclearable inbox for a given message type to a individual
    pub fn add_unclearable_inbox<M: Message, I: Individual + Recipient<M>> (&mut self) {
        self.add_inbox_helper::<M, I>(false);
    }

    fn add_inbox_helper<M: Message, I: Individual + Recipient<M>> (&mut self, clearable: bool) {
        let inbox = Inbox::<M>::new();

        // Gets short ID of individual
        let recipient_id = self.recipient_registry.get::<I>();

        let inbox_ptr = self.store_inbox(inbox, recipient_id);
        let individual_ptr = self.individuals[recipient_id].unwrap() as *mut I;

        // Create closure to process messages
        self.update_callbacks.push(Box::new(move || {
            unsafe {
                for packet in (*inbox_ptr).empty() {
                    (*individual_ptr).receive_packet(packet);
                }
            }
        }));

        // Create closure to empty messages without processing
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

    /// Store the inbox pointer of a given type into the routing array
    fn store_inbox<M: Message>(&mut self, inbox: Inbox<M>, recipient_type_id: usize) -> *mut Inbox<M> {
        let inbox_ptr = Box::into_raw(Box::new(inbox));
        // TODO: deallocate inbox at the end of times
        self.routing[recipient_type_id].as_mut().unwrap().add_new(inbox_ptr);
        inbox_ptr
    }

    /// Get the inbox pointer of a given type from the routing array
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

    /// Places message into the inbox of a given type
    fn send<M: Message>(&mut self, recipient: ID, message: M) {
        let packet = Packet{
            recipient_id: recipient,
            message: message
        };
        self.inbox_for(&packet).put(packet);
    }

    /// Process all messages and clear all inboxes afterwards
    pub fn process_messages(&mut self) {
        for callback in &self.update_callbacks {
            callback();
        }
    }

    /// Clear all inboxes without processing
    pub fn clear_all_clearable_messages(&mut self) {
        for callback in &self.clear_callbacks {
            callback();
        }
    }

    /// Process messages, up to 1000 layers of recursion
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