//! This is all auto-generated. Do not touch.
#![cfg_attr(rustfmt, rustfmt_skip)]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;



impl Actor for Family {
    type ID = FamilyID;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)] #[serde(transparent)]
pub struct FamilyID {
    _raw_id: RawID
}

impl TypedID for FamilyID {
    type Target = Family;

    fn from_raw(id: RawID) -> Self {
        FamilyID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl FamilyID {
    pub fn move_into(n_members: u32, home: BuildingID, time: TimeID, world: &mut World) -> Self {
        let id = FamilyID::from_raw(world.allocate_instance_id::<Family>());
        let swarm = world.local_broadcast::<Family>();
        world.send(swarm, MSG_Family_move_into(id, n_members, home, time));
        id
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Family_move_into(pub FamilyID, pub u32, pub BuildingID, pub TimeID);

impl Into<SleeperID> for FamilyID {
    fn into(self) -> SleeperID {
        SleeperID::from_raw(self.as_raw())
    }
}

impl Into<EvaluationRequesterID> for FamilyID {
    fn into(self) -> EvaluationRequesterID {
        EvaluationRequesterID::from_raw(self.as_raw())
    }
}

impl Into<TripListenerID> for FamilyID {
    fn into(self) -> TripListenerID {
        TripListenerID::from_raw(self.as_raw())
    }
}

impl Into<HouseholdID> for FamilyID {
    fn into(self) -> HouseholdID {
        HouseholdID::from_raw(self.as_raw())
    }
}

impl Into<TemporalID> for FamilyID {
    fn into(self) -> TemporalID {
        TemporalID::from_raw(self.as_raw())
    }
}

impl Into<RoughLocationID> for FamilyID {
    fn into(self) -> RoughLocationID {
        RoughLocationID::from_raw(self.as_raw())
    }
}

#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    
    SleeperID::register_implementor::<Family>(system);
    EvaluationRequesterID::register_implementor::<Family>(system);
    TripListenerID::register_implementor::<Family>(system);
    HouseholdID::register_implementor::<Family>(system);
    TemporalID::register_implementor::<Family>(system);
    RoughLocationID::register_implementor::<Family>(system);
    system.add_spawner::<Family, _, _>(
        |&MSG_Family_move_into(id, n_members, home, time), world| {
            Family::move_into(id, n_members, home, time, world)
        }, false
    );
}