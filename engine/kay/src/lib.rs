//! `Kay` is a high-performance actor system, suitable for simulating millions of entities.
//!
//! In `Kay`, actors concurrently send and receive asynchronous messages, but are
//! otherwise completely isloated from each other. Actors can only mutate their own state.
//!
//! Have a look at [`ActorSystem`](struct.ActorSystem.html),
//! [`ActorDefiner`](struct.ActorDefiner.html), [`World`](struct.World.html)
//! and [`Swarm`](swarm/struct.Swarm.html) to understand the main abstractions.
//!
//! Current Shortcomings:
//!
//! * Can't deal with messages to dead actors (undefined, often very confusing behaviour)

#![warn(missing_docs)]
#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]
#![feature(core_intrinsics)]
#![feature(optin_builtin_traits)]
#![feature(specialization)]
#![feature(conservative_impl_trait)]
#![feature(box_syntax)]
#![feature(nonzero)]
extern crate chunked;
extern crate compact;
#[macro_use]
extern crate compact_macros;
extern crate random;
extern crate core;

mod inbox;
mod slot_map;
pub mod swarm;
#[macro_use]
mod messaging;
mod type_registry;
mod id;
mod actor_system;
mod external;

pub use self::messaging::{Message, Packet, Fate};
pub use self::id::ID;
pub use self::actor_system::{ActorSystem, ActorDefiner, World};
pub use self::external::External;
