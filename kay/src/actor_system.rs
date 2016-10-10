use compact::Compact;
use swarm::Swarm;
use messaging::{Message, MessagePacket, Recipient};
use inbox::Inbox;
use std::ops::{Deref, DerefMut};

#[derive(Copy, Clone)]
pub struct ID {
    pub type_id: u16,
    pub version: u8,
    pub instance_id: u32
}

impl ID {
    pub fn invalid() -> ID {
        ID {
            type_id: u16::max_value(),
            version: u8::max_value(),
            instance_id: u32::max_value()
        }
    }

    pub fn individual(individual_type_id: usize) -> ID {
        ID {
            type_id: individual_type_id as u16,
            version: 0,
            instance_id: 0
        }
    }
}

pub trait Known {
    fn type_id() -> usize;
}

pub struct LivingActor<Actor: Compact> {
    pub id: ID,
    pub state: Actor
}

impl<Actor: Compact> Compact for LivingActor<Actor> {
    fn is_still_compact(&self) -> bool {self.state.is_still_compact()}
    fn dynamic_size_bytes(&self) -> usize {self.state.dynamic_size_bytes()}
    unsafe fn compact_from(&mut self, other: &Self, new_dynamic_part: *mut u8) {
        self.id = other.id;
        self.state.compact_from(&other.state, new_dynamic_part);
    }
}

impl<Actor: Compact> Deref for LivingActor<Actor> {
    type Target = Actor;

    fn deref(&self) -> &Actor {
        &self.state
    }
}


impl<Actor: Compact> DerefMut for LivingActor<Actor> {
    fn deref_mut(&mut self) -> &mut Actor {
        &mut self.state
    }
}

pub struct ActorSystem {
    routing: Vec<[Option<*mut u8>; 1024]>,
    swarms: [Option<*mut u8>; 1024],
    individuals: [Option<*mut u8>; 1024],
    update_callbacks: Vec<Box<Fn()>>,
}

impl ActorSystem {
    pub fn new() -> ActorSystem {
        let mut type_entries = Vec::with_capacity(1024);
        for _ in 0..1024 {
            type_entries.push([None; 1024]);
        }
        ActorSystem {
            routing: type_entries,
            swarms: [None; 1024],
            individuals: [None; 1024],
            update_callbacks: Vec::new()
        }
    }

    pub fn add_swarm<A: Compact + Known> (&mut self, swarm: Swarm<A>) {
        // containing system is now responsible
        self.swarms[A::type_id()] = Some(Box::into_raw(Box::new(swarm)) as *mut u8);
    }

    pub fn add_inbox<M: Message + 'static, A: Compact + Known + 'static>
        (&mut self, inbox: Inbox<M>)
        where A : Recipient<M> {
        let inbox_ptr = self.store_inbox(inbox, A::type_id());
        let swarm_ptr = self.swarms[A::type_id()].unwrap();
        let self_ptr = self as *mut Self;
        let m_typeid = M::type_id();
        let a_typeid = A::type_id();
        self.update_callbacks.push(Box::new(move || {
            unsafe {
                for packet in (*(inbox_ptr as *mut Inbox<M>)).empty() {
                    (*(swarm_ptr as *mut Swarm<A>))
                        .receive(
                            packet.recipient_id.instance_id as usize,
                            &packet.message,
                            &mut World{system: self_ptr}
                        );
                }
            }
        }))
    }

    pub fn add_individual<I>(&mut self, individual: I, individual_id: usize) {
        self.individuals[individual_id] = Some(Box::into_raw(Box::new(individual)) as *mut u8);
    }

    pub fn add_individual_inbox<M: Message, I>
        (&mut self, inbox: Inbox<M>, individual_id: usize)
        where I: Recipient<M> {
        let inbox_ptr = self.store_inbox(inbox, individual_id);
        let individual_ptr = self.individuals[individual_id].unwrap();
        let self_ptr = self as *mut Self;
        self.update_callbacks.push(Box::new(move || {
            unsafe {
                for packet in (*(inbox_ptr as *mut Inbox<M>)).empty() {
                    (*(individual_ptr as *mut I))
                        .receive(
                            &packet.message,
                            &mut World{system: self_ptr}
                        );
                }
            }
        }))
    }

    pub fn get_individual<I>(&self, individual_id: usize) -> &I {
        unsafe {
            &*(self.individuals[individual_id].unwrap() as *const I)
        }
    }

    pub fn get_individual_mut<I>(&mut self, individual_id: usize) -> &mut I {
        unsafe {
            &mut *(self.individuals[individual_id].unwrap() as *mut I)
        }
    }

    fn store_inbox<M: Message>(&mut self, inbox: Inbox<M>, recipient_type_id: usize) -> *mut u8 {
        let ref mut entry = self.routing[M::type_id()][recipient_type_id];
        assert!(entry.is_none());
        // containing router is now responsible
        let inbox_ptr = Box::into_raw(Box::new(inbox)) as *mut u8;
        *entry = Some(inbox_ptr);
        inbox_ptr
    }

    pub fn swarm<A: Compact + Known>(&mut self) -> &mut Swarm<A> {
        unsafe {
            &mut *(self.swarms[A::type_id()].unwrap() as *mut Swarm<A>)
        }
    }

    pub fn inbox_for<M: Message>(&mut self, packet: &MessagePacket<M>) -> &mut Inbox<M> {
        self.inbox_for_ids(M::type_id(), packet.recipient_id.type_id as usize)
    }

    pub fn inbox_for_ids<M: Message>(&mut self, message_type_id: usize, recipient_type_id: usize) -> &mut Inbox<M> {
        let ptr = self.routing[message_type_id][recipient_type_id].expect(
            &format!("No mailbox found: message type_id {}, recipient type_id {}", message_type_id, recipient_type_id));
        unsafe {
            let inbox: &mut Inbox<M> = &mut *(ptr as *mut Inbox<M>);
            inbox
        }
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

    pub fn world(&mut self) -> World {
        World{
            system: self
        }
    }
}

#[derive(Copy, Clone)]
pub struct World {
    system: *mut ActorSystem
}

impl World {
    pub fn send<M: Message>(&mut self, recipient: ID, message: M) {
        unsafe {
            (*self.system).send(recipient, message);
        }
    }
    pub fn create<A: Compact + Known>(&mut self, initial_state: A) -> LivingActor<A> {
        unsafe {
            (*self.system).swarm::<A>().create(initial_state)
        }
    }
    pub fn start<A: Compact + Known>(&mut self, living_actor: LivingActor<A>) {
        unsafe {
            (*self.system).swarm::<A>().add(&living_actor);
        }
    }
}