//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;



impl Actor for Mill {
    type ID = MillID;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)] #[serde(transparent)]
pub struct MillID {
    _raw_id: RawID
}

impl TypedID for MillID {
    type Target = Mill;

    fn from_raw(id: RawID) -> Self {
        MillID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl MillID {
    pub fn move_into(site: BuildingID, time: TimeID, world: &mut World) -> Self {
        let id = MillID::from_raw(world.allocate_instance_id::<Mill>());
        let swarm = world.local_broadcast::<Mill>();
        world.send(swarm, MSG_Mill_move_into(id, site, time));
        id
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Mill_move_into(pub MillID, pub BuildingID, pub TimeID);

impl Into<HouseholdID> for MillID {
    fn into(self) -> HouseholdID {
        HouseholdID::from_raw(self.as_raw())
    }
}

impl Into<TemporalID> for MillID {
    fn into(self) -> TemporalID {
        TemporalID::from_raw(self.as_raw())
    }
}

impl Into<SleeperID> for MillID {
    fn into(self) -> SleeperID {
        SleeperID::from_raw(self.as_raw())
    }
}

impl Into<EvaluationRequesterID> for MillID {
    fn into(self) -> EvaluationRequesterID {
        EvaluationRequesterID::from_raw(self.as_raw())
    }
}

impl Into<RoughLocationID> for MillID {
    fn into(self) -> RoughLocationID {
        RoughLocationID::from_raw(self.as_raw())
    }
}

impl Into<TripListenerID> for MillID {
    fn into(self) -> TripListenerID {
        TripListenerID::from_raw(self.as_raw())
    }
}

#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    
    HouseholdID::register_implementor::<Mill>(system);
    TemporalID::register_implementor::<Mill>(system);
    SleeperID::register_implementor::<Mill>(system);
    EvaluationRequesterID::register_implementor::<Mill>(system);
    RoughLocationID::register_implementor::<Mill>(system);
    TripListenerID::register_implementor::<Mill>(system);
    system.add_spawner::<Mill, _, _>(
        |&MSG_Mill_move_into(id, site, time), world| {
            Mill::move_into(id, site, time, world)
        }, false
    );
}