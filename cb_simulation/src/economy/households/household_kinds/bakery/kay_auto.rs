//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;



impl Actor for Bakery {
    type ID = BakeryID;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Serialize, Deserialize)] #[serde(transparent)]
pub struct BakeryID {
    _raw_id: RawID
}

impl Copy for BakeryID {}
impl Clone for BakeryID { fn clone(&self) -> Self { *self } }
impl ::std::fmt::Debug for BakeryID {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "BakeryID({:?})", self._raw_id)
    }
}
impl ::std::hash::Hash for BakeryID {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self._raw_id.hash(state);
    }
}
impl PartialEq for BakeryID {
    fn eq(&self, other: &BakeryID) -> bool {
        self._raw_id == other._raw_id
    }
}
impl Eq for BakeryID {}

impl TypedID for BakeryID {
    type Target = Bakery;

    fn from_raw(id: RawID) -> Self {
        BakeryID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl BakeryID {
    pub fn move_into(site: BuildingID, time: TimeID, world: &mut World) -> Self {
        let id = BakeryID::from_raw(world.allocate_instance_id::<Bakery>());
        let swarm = world.local_broadcast::<Bakery>();
        world.send(swarm, MSG_Bakery_move_into(id, site, time));
        id
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Bakery_move_into(pub BakeryID, pub BuildingID, pub TimeID);

impl Into<HouseholdID> for BakeryID {
    fn into(self) -> HouseholdID {
        HouseholdID::from_raw(self.as_raw())
    }
}

impl Into<TemporalID> for BakeryID {
    fn into(self) -> TemporalID {
        TemporalID::from_raw(self.as_raw())
    }
}

impl Into<SleeperID> for BakeryID {
    fn into(self) -> SleeperID {
        SleeperID::from_raw(self.as_raw())
    }
}

impl Into<EvaluationRequesterID> for BakeryID {
    fn into(self) -> EvaluationRequesterID {
        EvaluationRequesterID::from_raw(self.as_raw())
    }
}

impl Into<RoughLocationID> for BakeryID {
    fn into(self) -> RoughLocationID {
        RoughLocationID::from_raw(self.as_raw())
    }
}

impl Into<TripListenerID> for BakeryID {
    fn into(self) -> TripListenerID {
        TripListenerID::from_raw(self.as_raw())
    }
}

#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    
    HouseholdID::register_implementor::<Bakery>(system);
    TemporalID::register_implementor::<Bakery>(system);
    SleeperID::register_implementor::<Bakery>(system);
    EvaluationRequesterID::register_implementor::<Bakery>(system);
    RoughLocationID::register_implementor::<Bakery>(system);
    TripListenerID::register_implementor::<Bakery>(system);
    system.add_spawner::<Bakery, _, _>(
        |&MSG_Bakery_move_into(id, site, time), world| {
            Bakery::move_into(id, site, time, world)
        }, false
    );
}