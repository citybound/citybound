use super::compact::Compact;
use super::swarm::Swarm;
use super::messaging::{Message, Actor, Individual, MessagePacket, Recipient};
use super::inbox::Inbox;
use super::type_registry::TypeRegistry;
use std::ops::{Deref, DerefMut};
use std::intrinsics::{type_id, type_name};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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

pub struct LivingActor<A: Actor> {
    pub id: ID,
    pub state: A
}

impl<A: Actor> Compact for LivingActor<A> {
    fn is_still_compact(&self) -> bool {self.state.is_still_compact()}
    fn dynamic_size_bytes(&self) -> usize {self.state.dynamic_size_bytes()}
    unsafe fn compact_from(&mut self, other: &Self, new_dynamic_part: *mut u8) {
        self.id = other.id;
        self.state.compact_from(&other.state, new_dynamic_part);
    }
}

impl<A: Actor> Deref for LivingActor<A> {
    type Target = A;
    fn deref(&self) -> &A {&self.state}
}

impl<A: Actor> DerefMut for LivingActor<A> {
    fn deref_mut(&mut self) -> &mut A {&mut self.state}
}

const MAX_MESSAGE_TYPES : usize = 1024;
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
    swarms: [Option<*mut u8>; MAX_RECIPIENT_TYPES],
    individuals: [Option<*mut u8>; MAX_RECIPIENT_TYPES],
    update_callbacks: Vec<Box<Fn()>>,
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
            swarms: [None; MAX_RECIPIENT_TYPES],
            individuals: [None; MAX_RECIPIENT_TYPES],
            update_callbacks: Vec::new()
        }
    }

    pub fn add_swarm<A: Actor> (&mut self) {
        let swarm = Swarm::<A>::new();
        let recipient_id = self.recipient_registry.register_new::<A>();
        assert!(self.routing[recipient_id].is_none());
        self.routing[recipient_id] = Some(InboxMap::new());
        // TODO: deallocate swarm at the end of times
        self.swarms[recipient_id] = Some(Box::into_raw(Box::new(swarm)) as *mut u8);
    }

    pub fn add_inbox<M: Message, A: Actor> (&mut self) where Swarm<A>: Recipient<M> {
        let inbox = Inbox::<M>::new();
        let recipient_id = self.recipient_registry.get::<A>();
        let inbox_ptr = self.store_inbox(inbox, recipient_id);
        let swarm_ptr = self.swarms[recipient_id].unwrap() as *mut Swarm<A>;
        let self_ptr = self as *mut Self;
        self.update_callbacks.push(Box::new(move || {
            unsafe {
                for packet in (*inbox_ptr).empty() {
                    let swarm = &mut *(swarm_ptr);
                    let world = &mut World{system: self_ptr};
                    swarm.react_to(&packet.message, world, packet.recipient_id);
                }
            }
        }))
    }

    pub fn add_individual<I: Individual>(&mut self, individual: I) {
        let recipient_id = self.recipient_registry.register_new::<I>();
        assert!(self.routing[recipient_id].is_none());
        self.routing[recipient_id] = Some(InboxMap::new());
        self.individuals[recipient_id] = Some(Box::into_raw(Box::new(individual)) as *mut u8);
    }

    pub fn add_individual_inbox<M: Message, I: Individual + Recipient<M>> (&mut self) {
        let inbox = Inbox::<M>::new();
        let recipient_id = self.recipient_registry.get::<I>();
        let inbox_ptr = self.store_inbox(inbox, recipient_id);
        let individual_ptr = self.individuals[recipient_id].unwrap() as *mut I;
        let self_ptr = self as *mut Self;
        self.update_callbacks.push(Box::new(move || {
            unsafe {
                for packet in (*inbox_ptr).empty() {
                    (*individual_ptr)
                        .react_to(
                            &packet.message,
                            &mut World{system: self_ptr},
                            packet.recipient_id
                        );
                }
            }
        }))
    }

    pub fn get_individual<I: Individual>(&self) -> &I {
        unsafe {
            &*(self.individuals[self.recipient_registry.get::<I>()].unwrap() as *const I)
        }
    }

    pub fn get_individual_mut<I: Individual>(&mut self) -> &mut I {
        unsafe {
            &mut *(self.individuals[self.recipient_registry.get::<I>()].unwrap() as *mut I)
        }
    }

    pub fn individual_id<I: Individual>(&mut self) -> ID {
        ID::individual(self.recipient_registry.get::<I>())
    }

    pub fn broadcast_id<A: Actor>(&mut self) -> ID {
        ID::broadcast(self.recipient_registry.get::<A>())
    }

    fn store_inbox<M: Message>(&mut self, inbox: Inbox<M>, recipient_type_id: usize) -> *mut Inbox<M> {
        let inbox_ptr = Box::into_raw(Box::new(inbox));
        // TODO: deallocate inbox at the end of times
        self.routing[recipient_type_id].as_mut().unwrap().add_new(inbox_ptr);
        inbox_ptr
    }

    pub fn create<A: Actor>(&mut self, initial_state: A) -> LivingActor<A> {
        let recipient_id = self.recipient_registry.get::<A>();
        let swarm = unsafe {&mut *(self.swarms[recipient_id].unwrap() as *mut Swarm<A>)};
        LivingActor{
            state: initial_state,
            id: ID{
                type_id: recipient_id as u16,
                version: 0,
                instance_id: swarm.allocate_instance_id()
            }
        }
    }

    pub fn start<A: Actor>(&mut self, living_actor: LivingActor<A>) {
        let recipient_id = self.recipient_registry.get::<A>();
        let swarm = unsafe {&mut *(self.swarms[recipient_id].unwrap() as *mut Swarm<A>)};
        swarm.add(&living_actor);
    }

    pub fn inbox_for<M: Message>(&mut self, packet: &MessagePacket<M>) -> &mut Inbox<M> {
        let inbox_ptr = self.routing[packet.recipient_id.type_id as usize].as_ref()
            .expect("Recipient not found")
            .get::<M>()
            .expect(format!("Inbox for {} not found for {}", unsafe{type_name::<M>()}, self.recipient_registry.get_name(packet.recipient_id.type_id as usize)).as_str());
        unsafe{&mut *inbox_ptr}
    }

    fn send<M: Message>(&mut self, recipient: ID, message: M) {
        let packet = MessagePacket{
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

    pub fn process_all_messages(&mut self) {
        for _i in 0..1000 {
            self.process_messages();
        }
    }

    pub fn world(&mut self) -> World {
        World{
            system: self
        }
    }
}

impl Default for ActorSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Copy, Clone)]
pub struct World {
    system: *mut ActorSystem
}

impl World {
    pub fn send<M: Message>(&mut self, recipient: ID, message: M) {
        unsafe {(*self.system).send(recipient, message)};
    }
    pub fn send_to_individual<I: Individual, M: Message>(&mut self, message: M) {
        unsafe {
            self.send((*self.system).individual_id::<I>(), message);
        }
    }
    // pub fn broadcast<Recipient, M: Message>(&mut self, message: M) {
    //     self.send(ID::broadcast::<Recipient>(), message);
    // }
    pub fn create<A: Actor>(&mut self, initial_state: A) -> LivingActor<A> {
        unsafe {(*self.system).create(initial_state)}
    }
    pub fn start<A: Actor>(&mut self, living_actor: LivingActor<A>) {
        unsafe {(*self.system).start(living_actor)};
    }
}