//! `Kay` is a high-performance actor system, suitable for simulating millions of entities.
//!
//! In `Kay`, actors concurrently send and receive asynchronous messages, but are
//! otherwise completely isloated from each other. Actors can only mutate their own state.
//!
//! Have a look at [`Actor`](trait.Actor.html), [`Recipient`](trait.Recipient.html)
//! and [`Swarm`](struct.Swarm.html) to understand the main abstractions.

#![warn(missing_docs)]
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
pub mod swarm;
#[macro_use]
mod messaging;
mod type_registry;
mod id;
mod actor_system;

pub use self::messaging::{Message, Packet, Recipient, Fate};
pub use self::id::ID;
pub use self::actor_system::{THE_SYSTEM, ActorSystem, Actor};
