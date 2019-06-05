//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;





impl CBPrototypeKindID {
    
}



impl Into<PrototypeKindID> for CBPrototypeKindID {
    fn into(self) -> PrototypeKindID {
        PrototypeKindID::from_raw(self.as_raw())
    }
}

#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    
    PrototypeKindID::register_implementor::<CBPrototypeKind>(system);
}