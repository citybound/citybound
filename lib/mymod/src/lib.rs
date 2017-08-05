#![feature(plugin)]
#![plugin(clippy)]

#[macro_use]
extern crate weaver;

use weaver::CityboundMod;
use weaver::kay::ActorSystem;

struct MyMod;

impl CityboundMod for MyMod {
    fn setup(_system: &mut ActorSystem) -> MyMod {
        println!("my mod was loaded!");
        MyMod
    }
}

register_module! {
    module: MyMod,
}
