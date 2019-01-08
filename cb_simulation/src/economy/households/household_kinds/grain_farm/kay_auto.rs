//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;



impl Actor for GrainFarm {
    type ID = GrainFarmID;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)] #[serde(transparent)]
pub struct GrainFarmID {
    _raw_id: RawID
}

impl TypedID for GrainFarmID {
    type Target = GrainFarm;

    fn from_raw(id: RawID) -> Self {
        GrainFarmID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl GrainFarmID {
    pub fn move_into(site: BuildingID, time: TimeID, world: &mut World) -> Self {
        let id = GrainFarmID::from_raw(world.allocate_instance_id::<GrainFarm>());
        let swarm = world.local_broadcast::<GrainFarm>();
        world.send(swarm, MSG_GrainFarm_move_into(id, site, time));
        id
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_GrainFarm_move_into(pub GrainFarmID, pub BuildingID, pub TimeID);

impl Into<HouseholdID> for GrainFarmID {
    fn into(self) -> HouseholdID {
        HouseholdID::from_raw(self.as_raw())
    }
}

impl Into<TemporalID> for GrainFarmID {
    fn into(self) -> TemporalID {
        TemporalID::from_raw(self.as_raw())
    }
}

impl Into<SleeperID> for GrainFarmID {
    fn into(self) -> SleeperID {
        SleeperID::from_raw(self.as_raw())
    }
}

impl Into<EvaluationRequesterID> for GrainFarmID {
    fn into(self) -> EvaluationRequesterID {
        EvaluationRequesterID::from_raw(self.as_raw())
    }
}

impl Into<RoughLocationID> for GrainFarmID {
    fn into(self) -> RoughLocationID {
        RoughLocationID::from_raw(self.as_raw())
    }
}

impl Into<TripListenerID> for GrainFarmID {
    fn into(self) -> TripListenerID {
        TripListenerID::from_raw(self.as_raw())
    }
}

#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    
    HouseholdID::register_implementor::<GrainFarm>(system);
    TemporalID::register_implementor::<GrainFarm>(system);
    SleeperID::register_implementor::<GrainFarm>(system);
    EvaluationRequesterID::register_implementor::<GrainFarm>(system);
    RoughLocationID::register_implementor::<GrainFarm>(system);
    TripListenerID::register_implementor::<GrainFarm>(system);
    system.add_spawner::<GrainFarm, _, _>(
        |&MSG_GrainFarm_move_into(id, site, time), world| {
            GrainFarm::move_into(id, site, time, world)
        }, false
    );
}