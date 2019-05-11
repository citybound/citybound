//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;



impl Actor for VegetableFarm {
    type ID = VegetableFarmID;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Serialize, Deserialize)] #[serde(transparent)]
pub struct VegetableFarmID {
    _raw_id: RawID
}

impl Copy for VegetableFarmID {}
impl Clone for VegetableFarmID { fn clone(&self) -> Self { *self } }
impl ::std::fmt::Debug for VegetableFarmID {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "VegetableFarmID({:?})", self._raw_id)
    }
}
impl ::std::hash::Hash for VegetableFarmID {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self._raw_id.hash(state);
    }
}
impl PartialEq for VegetableFarmID {
    fn eq(&self, other: &VegetableFarmID) -> bool {
        self._raw_id == other._raw_id
    }
}
impl Eq for VegetableFarmID {}

impl TypedID for VegetableFarmID {
    type Target = VegetableFarm;

    fn from_raw(id: RawID) -> Self {
        VegetableFarmID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl VegetableFarmID {
    pub fn move_into(site: BuildingID, time: TimeID, world: &mut World) -> Self {
        let id = VegetableFarmID::from_raw(world.allocate_instance_id::<VegetableFarm>());
        let swarm = world.local_broadcast::<VegetableFarm>();
        world.send(swarm, MSG_VegetableFarm_move_into(id, site, time));
        id
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_VegetableFarm_move_into(pub VegetableFarmID, pub BuildingID, pub TimeID);

impl Into<HouseholdID> for VegetableFarmID {
    fn into(self) -> HouseholdID {
        HouseholdID::from_raw(self.as_raw())
    }
}

impl Into<TemporalID> for VegetableFarmID {
    fn into(self) -> TemporalID {
        TemporalID::from_raw(self.as_raw())
    }
}

impl Into<SleeperID> for VegetableFarmID {
    fn into(self) -> SleeperID {
        SleeperID::from_raw(self.as_raw())
    }
}

impl Into<EvaluationRequesterID> for VegetableFarmID {
    fn into(self) -> EvaluationRequesterID {
        EvaluationRequesterID::from_raw(self.as_raw())
    }
}

impl Into<RoughLocationID> for VegetableFarmID {
    fn into(self) -> RoughLocationID {
        RoughLocationID::from_raw(self.as_raw())
    }
}

impl Into<TripListenerID> for VegetableFarmID {
    fn into(self) -> TripListenerID {
        TripListenerID::from_raw(self.as_raw())
    }
}

#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    
    HouseholdID::register_implementor::<VegetableFarm>(system);
    TemporalID::register_implementor::<VegetableFarm>(system);
    SleeperID::register_implementor::<VegetableFarm>(system);
    EvaluationRequesterID::register_implementor::<VegetableFarm>(system);
    RoughLocationID::register_implementor::<VegetableFarm>(system);
    TripListenerID::register_implementor::<VegetableFarm>(system);
    system.add_spawner::<VegetableFarm, _, _>(
        |&MSG_VegetableFarm_move_into(id, site, time), world| {
            VegetableFarm::move_into(id, site, time, world)
        }, false
    );
}