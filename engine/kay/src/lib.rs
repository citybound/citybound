//! `Kay` is a high-performance actor system, suitable for simulating millions of entities.
//!
//! In `Kay`, actors concurrently send and receive asynchronous messages, but are
//! otherwise completely isloated from each other. Actors can only mutate their own state.
//!
//! Have a look at [`ActorSystem`](struct.ActorSystem.html), [`World`](struct.World.html)
//! and [`Swarm`](swarm/struct.Swarm.html) to understand the main abstractions.
//!
//! Current Shortcomings:
//!
//! * Can't deal with messages to dead actors (undefined, often very confusing behaviour)

#![warn(missing_docs)]
#![feature(core_intrinsics)]
#![feature(optin_builtin_traits)]
#![feature(specialization)]
#![feature(box_syntax)]
#![feature(nonzero)]
#![feature(tcpstream_connect_timeout)]
extern crate chunky;
extern crate compact;
#[macro_use]
extern crate compact_macros;
extern crate core;
extern crate byteorder;

mod inbox;
mod slot_map;
mod swarm;
mod messaging;
mod type_registry;
mod id;
mod actor_system;
mod networking;
mod external;

pub use self::messaging::{Message, Packet, Fate};
pub use self::id::{RawID, TypedID, MachineID};
pub use self::actor_system::{Actor, ActorSystem, World, TraitIDFrom};
pub use self::networking::Networking;
pub use self::external::External;
