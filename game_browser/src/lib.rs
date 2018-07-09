#![feature(proc_macro)]

#[macro_use]
extern crate stdweb;
use stdweb::js_export;

extern crate citybound_common;
use citybound_common::*;

#[js_export]
pub fn test() {
    js!{ console.log("Before setup") }
    let mut system = kay::ActorSystem::new(kay::Networking::new(0, vec![]));
    setup_all(&mut system);
    js!{ console.log("After setup") }
}
