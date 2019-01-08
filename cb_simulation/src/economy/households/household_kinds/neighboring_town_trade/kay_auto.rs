//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;



impl Actor for NeighboringTownTrade {
    type ID = NeighboringTownTradeID;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)] #[serde(transparent)]
pub struct NeighboringTownTradeID {
    _raw_id: RawID
}

impl TypedID for NeighboringTownTradeID {
    type Target = NeighboringTownTrade;

    fn from_raw(id: RawID) -> Self {
        NeighboringTownTradeID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl NeighboringTownTradeID {
    pub fn move_into(town: BuildingID, time: TimeID, world: &mut World) -> Self {
        let id = NeighboringTownTradeID::from_raw(world.allocate_instance_id::<NeighboringTownTrade>());
        let swarm = world.local_broadcast::<NeighboringTownTrade>();
        world.send(swarm, MSG_NeighboringTownTrade_move_into(id, town, time));
        id
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_NeighboringTownTrade_move_into(pub NeighboringTownTradeID, pub BuildingID, pub TimeID);

impl Into<HouseholdID> for NeighboringTownTradeID {
    fn into(self) -> HouseholdID {
        HouseholdID::from_raw(self.as_raw())
    }
}

impl Into<SleeperID> for NeighboringTownTradeID {
    fn into(self) -> SleeperID {
        SleeperID::from_raw(self.as_raw())
    }
}

impl Into<EvaluationRequesterID> for NeighboringTownTradeID {
    fn into(self) -> EvaluationRequesterID {
        EvaluationRequesterID::from_raw(self.as_raw())
    }
}

impl Into<TripListenerID> for NeighboringTownTradeID {
    fn into(self) -> TripListenerID {
        TripListenerID::from_raw(self.as_raw())
    }
}

impl Into<TemporalID> for NeighboringTownTradeID {
    fn into(self) -> TemporalID {
        TemporalID::from_raw(self.as_raw())
    }
}

impl Into<RoughLocationID> for NeighboringTownTradeID {
    fn into(self) -> RoughLocationID {
        RoughLocationID::from_raw(self.as_raw())
    }
}

#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    
    HouseholdID::register_implementor::<NeighboringTownTrade>(system);
    SleeperID::register_implementor::<NeighboringTownTrade>(system);
    EvaluationRequesterID::register_implementor::<NeighboringTownTrade>(system);
    TripListenerID::register_implementor::<NeighboringTownTrade>(system);
    TemporalID::register_implementor::<NeighboringTownTrade>(system);
    RoughLocationID::register_implementor::<NeighboringTownTrade>(system);
    system.add_spawner::<NeighboringTownTrade, _, _>(
        |&MSG_NeighboringTownTrade_move_into(id, town, time), world| {
            NeighboringTownTrade::move_into(id, town, time, world)
        }, false
    );
}