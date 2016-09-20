#![allow(dead_code)]

mod tagged_relative_pointer;
mod allocators;
#[macro_use]
mod embedded;
mod chunked;
mod inbox;
mod slot_map;
mod swarm;
mod messaging;
use embedded::{Embedded, EmbeddedVec};
use chunked::{MemChunker};
use swarm::Swarm;
use inbox::{Inbox};
use messaging::{Message, Recipient, ActorSystem};

#[derive(Copy, Clone)]
pub struct ID {
    type_id: u16,
    version: u8,
    instance_id: u32
}

pub trait ShortTypeId {
    fn type_id() -> usize;
}

pub struct Actor<ActorState: Embedded>{
    id: ID,
    state: ActorState
}

impl<ActorState: Embedded> Embedded for Actor<ActorState> {
    fn is_still_embedded(&self) -> bool {self.state.is_still_embedded()}
    fn dynamic_size_bytes(&self) -> usize {self.state.dynamic_size_bytes()}
    unsafe fn embed_from(&mut self, other: &Self, new_dynamic_part: *mut u8) {
        self.id = other.id;
        self.state.embed_from(&other.state, new_dynamic_part);
    }
}

// ----------

derive_embedded!{
struct Test {
    a: u32,
    b: u16,
    x: EmbeddedVec<u8>,
    y: EmbeddedVec<u16>
}
}

derive_embedded!{
struct AddX {
    x: u8
}
}

impl ShortTypeId for AddX {
    fn type_id() -> usize {42}
}

impl Message for AddX {}

impl Recipient<AddX> for Actor<Test> {
    fn receive(&mut self, message: &AddX) {
        self.state.x.push(message.x);
    }
}

impl ShortTypeId for Actor<Test> {
    fn type_id() -> usize {13}
}



fn main () {
    let mut system = ActorSystem::new();

    system.add_swarm::<Test>(Swarm::new(MemChunker::new("test_actors", 512), 30));    
    system.add_inbox::<AddX, Test>(Inbox::new(MemChunker::new("add_x", 512), 4));

    let actor = {
        let swarm = system.swarm::<Test>();

        let actor = swarm.create(Test {
            a: 1,
            b: 2,
            x: EmbeddedVec::new(),
            y: EmbeddedVec::new()
        });

        swarm.add(&actor);

        actor
    };

    system.send(AddX{x: 99}, actor.id);
    system.send(AddX{x: 77}, actor.id);

    system.process_messages(); 

    {
        let swarm = system.swarm::<Test>();
        println!("{}, {}, {}", swarm.at(0).state.x.len(), swarm.at(0).state.x[0], swarm.at(0).state.x[1]);
        println!("done!");
    }
}