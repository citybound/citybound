//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;



impl Actor for CBPlanningLogic {
    type ID = CBPlanningLogicID;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Serialize, Deserialize)] #[serde(transparent)]
pub struct CBPlanningLogicID {
    _raw_id: RawID
}

impl Copy for CBPlanningLogicID {}
impl Clone for CBPlanningLogicID { fn clone(&self) -> Self { *self } }
impl ::std::fmt::Debug for CBPlanningLogicID {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "CBPlanningLogicID({:?})", self._raw_id)
    }
}
impl ::std::hash::Hash for CBPlanningLogicID {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self._raw_id.hash(state);
    }
}
impl PartialEq for CBPlanningLogicID {
    fn eq(&self, other: &CBPlanningLogicID) -> bool {
        self._raw_id == other._raw_id
    }
}
impl Eq for CBPlanningLogicID {}

impl TypedID for CBPlanningLogicID {
    type Target = CBPlanningLogic;

    fn from_raw(id: RawID) -> Self {
        CBPlanningLogicID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl CBPlanningLogicID {
    
}



impl Into<PlanningLogicID> for CBPlanningLogicID {
    fn into(self) -> PlanningLogicID {
        PlanningLogicID::from_raw(self.as_raw())
    }
}


impl CBGestureIntentID {
    
}



impl Into<GestureIntentID> for CBGestureIntentID {
    fn into(self) -> GestureIntentID {
        GestureIntentID::from_raw(self.as_raw())
    }
}


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
    
    PlanningLogicID::register_implementor::<CBPlanningLogic>(system);
    
    GestureIntentID::register_implementor::<CBGestureIntent>(system);
    
    PrototypeKindID::register_implementor::<CBPrototypeKind>(system);
}