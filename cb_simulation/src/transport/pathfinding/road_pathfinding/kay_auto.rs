//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;





impl LaneID {
    
}



impl Into<LinkID> for LaneID {
    fn into(self) -> LinkID {
        LinkID::from_raw(self.as_raw())
    }
}

impl Into<RoughLocationID> for LaneID {
    fn into(self) -> RoughLocationID {
        RoughLocationID::from_raw(self.as_raw())
    }
}

#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    
    LinkID::register_implementor::<Lane>(system);
    RoughLocationID::register_implementor::<Lane>(system);
}