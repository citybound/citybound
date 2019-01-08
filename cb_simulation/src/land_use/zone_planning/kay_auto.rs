//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;





impl LandUseID {
    
}



impl Into<std::fmt::DisplayID> for LandUseID {
    fn into(self) -> std::fmt::DisplayID {
        std::fmt::DisplayID::from_raw(self.as_raw())
    }
}

#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    
    std::fmt::DisplayID::register_implementor::<LandUse>(system);
}