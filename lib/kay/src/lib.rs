#![feature(plugin)]
#![plugin(clippy)]
#![feature(core_intrinsics)]
#![feature(optin_builtin_traits)]
#![feature(specialization)]
#![feature(conservative_impl_trait)]
#![feature(box_syntax)]
#![feature(nonzero)]
#![allow(no_effect)]
extern crate chunked;
extern crate compact;
#[macro_use]
extern crate compact_macros;
extern crate random;
extern crate core;

mod inbox;
mod slot_map;
mod swarm;
#[macro_use]
mod messaging;
mod type_registry;
mod id;
mod actor_system;

pub use self::chunked::MemChunker;
pub use self::swarm::{Swarm, Create, CreateWith, RecipientAsSwarm, ToRandom, RequestConfirmation,
                      Confirmation};
pub use self::inbox::Inbox;
pub use self::messaging::{Message, Packet, Actor, Recipient, Individual, Fate};
pub use self::id::ID;
pub use self::actor_system::{THE_SYSTEM, ActorSystem};
