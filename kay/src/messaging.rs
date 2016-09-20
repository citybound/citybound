use {ID, ShortTypeId, Actor};
use embedded::Embedded;
use inbox::Inbox;
use swarm::Swarm;

pub trait Recipient<M: Message> : ShortTypeId {
    fn receive(&mut self, message: &M);
}

pub trait Message : Embedded + ShortTypeId {}

pub struct MessagePacket<M: Message> {
    pub recipient_id: ID,
    pub message: M
}

impl<M: Message> Embedded for MessagePacket<M> {
    fn is_still_embedded(&self) -> bool {self.message.is_still_embedded()}
    fn dynamic_size_bytes(&self) -> usize {self.message.dynamic_size_bytes()}
    unsafe fn embed_from(&mut self, source: &Self, new_dynamic_part: *mut u8) {
        self.recipient_id = source.recipient_id;
        self.message.embed_from(&source.message, new_dynamic_part);
    }
}

pub struct ActorSystem {
    routing: Vec<[Option<*mut u8>; 1024]>,
    swarms: [Option<*mut u8>; 1024],
    update_callbacks: Vec<Box<Fn()>>
}

impl ActorSystem {
    pub fn new() -> ActorSystem {
        let mut type_entries = Vec::with_capacity(1024);
        for _ in 0..1024 {
            type_entries.push([None; 1024]);
        }
        ActorSystem{
            routing: type_entries,
            swarms: [None; 1024],
            update_callbacks: Vec::new()
        }
    }

    pub fn add_swarm<S: Embedded> (&mut self, swarm: Swarm<Actor<S>>)
        where Actor<S> : ShortTypeId {
        // containing router is now responsible
        self.swarms[Actor::<S>::type_id()] = Some(Box::into_raw(Box::new(swarm)) as *mut u8);
    }

    pub fn add_inbox<M: Message + 'static, S: Embedded + 'static>
        (&mut self, inbox: Inbox<M>)
        where Actor<S> : Recipient<M> {
        let ref mut entry = self.routing[M::type_id()][Actor::<S>::type_id()];
        assert!(entry.is_none());
        // containing router is now responsible
        let inbox_ptr = Box::into_raw(Box::new(inbox)) as *mut u8;
        let swarm_ptr = self.swarms[Actor::<S>::type_id()].unwrap();
        *entry = Some(inbox_ptr);
        self.update_callbacks.push(Box::new(move || {
            unsafe {
                for packet in (*(inbox_ptr as *mut Inbox<M>)).empty() {
                    (*(swarm_ptr as *mut Swarm<Actor<S>>))
                        .receive(packet.recipient_id.instance_id as usize, &packet.message);
                }
            }
        }))
    }

    pub fn swarm<S: Embedded>(&mut self) -> &mut Swarm<Actor<S>> where Actor<S> : ShortTypeId  {
        unsafe {
            &mut *(self.swarms[Actor::<S>::type_id()].unwrap() as *mut Swarm<Actor<S>>)
        }
    }

    pub fn inbox_for<M: Message>(&mut self, packet: &MessagePacket<M>) -> &mut Inbox<M> {
        self.inbox_for_ids(M::type_id(), packet.recipient_id.type_id as usize)
    }

    pub fn inbox_for_ids<M: Message>(&mut self, message_type_id: usize, recipient_type_id: usize) -> &mut Inbox<M> {
        let ptr = self.routing[message_type_id][recipient_type_id].unwrap();
        unsafe {
            let inbox: &mut Inbox<M> = &mut *(ptr as *mut Inbox<M>);
            inbox
        }
    }

    pub fn send<M: Message>(&mut self, message: M, recipient: ID) {
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
}