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
mod actor_system;
pub use embedded::{Embedded, EmbeddedVec as EVec};
pub use chunked::{MemChunker};
pub use swarm::Swarm;
pub use inbox::{Inbox};
pub use messaging::{Message, Recipient};
pub use actor_system::{ID, Known, LivingActor, ActorSystem, SystemServices};