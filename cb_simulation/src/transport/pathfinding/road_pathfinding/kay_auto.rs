//! This is all auto-generated. Do not touch.
#![cfg_attr(rustfmt, rustfmt_skip)]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;





impl LaneID {
    
}



impl Into<NodeID> for LaneID {
    fn into(self) -> NodeID {
        NodeID::from_raw(self.as_raw())
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
    
    NodeID::register_implementor::<Lane>(system);
    RoughLocationID::register_implementor::<Lane>(system);
}