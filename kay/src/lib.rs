#![allow(dead_code)]

mod tagged_relative_pointer;
mod allocators;
#[macro_use]
mod compact;
mod chunked;
mod inbox;
mod slot_map;
mod swarm;
mod messaging;
mod actor_system;
pub use compact::{Compact, CompactVec as CVec};
pub use chunked::{MemChunker};
pub use swarm::Swarm;
pub use inbox::{Inbox};
pub use messaging::{Message, Recipient};
pub use actor_system::{ID, Known, LivingActor, ActorSystem, World};