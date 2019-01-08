//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;



impl Actor for CowFarm {
    type ID = CowFarmID;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)] #[serde(transparent)]
pub struct CowFarmID {
    _raw_id: RawID
}

impl TypedID for CowFarmID {
    type Target = CowFarm;

    fn from_raw(id: RawID) -> Self {
        CowFarmID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl CowFarmID {
    pub fn move_into(site: BuildingID, time: TimeID, world: &mut World) -> Self {
        let id = CowFarmID::from_raw(world.allocate_instance_id::<CowFarm>());
        let swarm = world.local_broadcast::<CowFarm>();
        world.send(swarm, MSG_CowFarm_move_into(id, site, time));
        id
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_CowFarm_move_into(pub CowFarmID, pub BuildingID, pub TimeID);

impl Into<HouseholdID> for CowFarmID {
    fn into(self) -> HouseholdID {
        HouseholdID::from_raw(self.as_raw())
    }
}

impl Into<TemporalID> for CowFarmID {
    fn into(self) -> TemporalID {
        TemporalID::from_raw(self.as_raw())
    }
}

impl Into<SleeperID> for CowFarmID {
    fn into(self) -> SleeperID {
        SleeperID::from_raw(self.as_raw())
    }
}

impl Into<EvaluationRequesterID> for CowFarmID {
    fn into(self) -> EvaluationRequesterID {
        EvaluationRequesterID::from_raw(self.as_raw())
    }
}

impl Into<RoughLocationID> for CowFarmID {
    fn into(self) -> RoughLocationID {
        RoughLocationID::from_raw(self.as_raw())
    }
}

impl Into<TripListenerID> for CowFarmID {
    fn into(self) -> TripListenerID {
        TripListenerID::from_raw(self.as_raw())
    }
}

#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    
    HouseholdID::register_implementor::<CowFarm>(system);
    TemporalID::register_implementor::<CowFarm>(system);
    SleeperID::register_implementor::<CowFarm>(system);
    EvaluationRequesterID::register_implementor::<CowFarm>(system);
    RoughLocationID::register_implementor::<CowFarm>(system);
    TripListenerID::register_implementor::<CowFarm>(system);
    system.add_spawner::<CowFarm, _, _>(
        |&MSG_CowFarm_move_into(id, site, time), world| {
            CowFarm::move_into(id, site, time, world)
        }, false
    );
}