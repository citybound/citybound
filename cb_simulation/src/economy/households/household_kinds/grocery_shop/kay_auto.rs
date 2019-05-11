//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;



impl Actor for GroceryShop {
    type ID = GroceryShopID;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Serialize, Deserialize)] #[serde(transparent)]
pub struct GroceryShopID {
    _raw_id: RawID
}

impl Copy for GroceryShopID {}
impl Clone for GroceryShopID { fn clone(&self) -> Self { *self } }
impl ::std::fmt::Debug for GroceryShopID {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "GroceryShopID({:?})", self._raw_id)
    }
}
impl ::std::hash::Hash for GroceryShopID {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self._raw_id.hash(state);
    }
}
impl PartialEq for GroceryShopID {
    fn eq(&self, other: &GroceryShopID) -> bool {
        self._raw_id == other._raw_id
    }
}
impl Eq for GroceryShopID {}

impl TypedID for GroceryShopID {
    type Target = GroceryShop;

    fn from_raw(id: RawID) -> Self {
        GroceryShopID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl GroceryShopID {
    pub fn move_into(site: BuildingID, time: TimeID, world: &mut World) -> Self {
        let id = GroceryShopID::from_raw(world.allocate_instance_id::<GroceryShop>());
        let swarm = world.local_broadcast::<GroceryShop>();
        world.send(swarm, MSG_GroceryShop_move_into(id, site, time));
        id
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_GroceryShop_move_into(pub GroceryShopID, pub BuildingID, pub TimeID);

impl Into<HouseholdID> for GroceryShopID {
    fn into(self) -> HouseholdID {
        HouseholdID::from_raw(self.as_raw())
    }
}

impl Into<EvaluationRequesterID> for GroceryShopID {
    fn into(self) -> EvaluationRequesterID {
        EvaluationRequesterID::from_raw(self.as_raw())
    }
}

impl Into<TemporalID> for GroceryShopID {
    fn into(self) -> TemporalID {
        TemporalID::from_raw(self.as_raw())
    }
}

impl Into<SleeperID> for GroceryShopID {
    fn into(self) -> SleeperID {
        SleeperID::from_raw(self.as_raw())
    }
}

impl Into<RoughLocationID> for GroceryShopID {
    fn into(self) -> RoughLocationID {
        RoughLocationID::from_raw(self.as_raw())
    }
}

impl Into<TripListenerID> for GroceryShopID {
    fn into(self) -> TripListenerID {
        TripListenerID::from_raw(self.as_raw())
    }
}

#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    
    HouseholdID::register_implementor::<GroceryShop>(system);
    EvaluationRequesterID::register_implementor::<GroceryShop>(system);
    TemporalID::register_implementor::<GroceryShop>(system);
    SleeperID::register_implementor::<GroceryShop>(system);
    RoughLocationID::register_implementor::<GroceryShop>(system);
    TripListenerID::register_implementor::<GroceryShop>(system);
    system.add_spawner::<GroceryShop, _, _>(
        |&MSG_GroceryShop_move_into(id, site, time), world| {
            GroceryShop::move_into(id, site, time, world)
        }, false
    );
}