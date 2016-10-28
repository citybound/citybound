#![allow(dead_code)]
mod pointer_to_maybe_compact;
mod allocators;
#[macro_use]
mod compact;
mod compact_vec;
mod compact_dict;
mod chunked;
mod inbox;
mod slot_map;
mod swarm;
#[macro_use]
mod messaging;
mod type_registry;
mod actor_system;
pub use self::compact::{Compact};
pub use self::compact_vec::{CompactVec as CVec};
pub use self::compact_dict::{CompactDict as CDict};
pub use self::chunked::{MemChunker};
pub use self::swarm::Swarm;
pub use self::inbox::{Inbox};
pub use self::messaging::{Message, Recipient};
pub use self::actor_system::{ID, LivingActor, ActorSystem, World};
pub use self::actor_system::Storage::{InMemory, OnDisk};