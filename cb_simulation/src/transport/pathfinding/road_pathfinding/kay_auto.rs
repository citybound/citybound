//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;





impl CarLaneID {
    
}



impl Into<LinkID> for CarLaneID {
    fn into(self) -> LinkID {
        LinkID::from_raw(self.as_raw())
    }
}

impl Into<RoughLocationID> for CarLaneID {
    fn into(self) -> RoughLocationID {
        RoughLocationID::from_raw(self.as_raw())
    }
}


impl SidewalkID {
    
}



impl Into<LinkID> for SidewalkID {
    fn into(self) -> LinkID {
        LinkID::from_raw(self.as_raw())
    }
}

impl Into<RoughLocationID> for SidewalkID {
    fn into(self) -> RoughLocationID {
        RoughLocationID::from_raw(self.as_raw())
    }
}

#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    
    LinkID::register_implementor::<CarLane>(system);
    RoughLocationID::register_implementor::<CarLane>(system);
    
    LinkID::register_implementor::<Sidewalk>(system);
    RoughLocationID::register_implementor::<Sidewalk>(system);
}