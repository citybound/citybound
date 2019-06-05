//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;

#[derive(Serialize, Deserialize)] #[serde(transparent)]
pub struct EvaluationRequesterID {
    _raw_id: RawID
}

impl Copy for EvaluationRequesterID {}
impl Clone for EvaluationRequesterID { fn clone(&self) -> Self { *self } }
impl ::std::fmt::Debug for EvaluationRequesterID {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "EvaluationRequesterID({:?})", self._raw_id)
    }
}
impl ::std::hash::Hash for EvaluationRequesterID {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self._raw_id.hash(state);
    }
}
impl PartialEq for EvaluationRequesterID {
    fn eq(&self, other: &EvaluationRequesterID) -> bool {
        self._raw_id == other._raw_id
    }
}
impl Eq for EvaluationRequesterID {}

pub struct EvaluationRequesterRepresentative;

impl ActorOrActorTrait for EvaluationRequesterRepresentative {
    type ID = EvaluationRequesterID;
}

impl TypedID for EvaluationRequesterID {
    type Target = EvaluationRequesterRepresentative;

    fn from_raw(id: RawID) -> Self {
        EvaluationRequesterID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl<Act: Actor + EvaluationRequester> TraitIDFrom<Act> for EvaluationRequesterID {}

impl EvaluationRequesterID {
    pub fn expect_n_results(self, resource: Resource, n: u32, world: &mut World) {
        world.send(self.as_raw(), MSG_EvaluationRequester_expect_n_results(resource, n));
    }
    
    pub fn on_result(self, result: EvaluatedSearchResult, world: &mut World) {
        world.send(self.as_raw(), MSG_EvaluationRequester_on_result(result));
    }

    pub fn register_trait(system: &mut ActorSystem) {
        system.register_trait::<EvaluationRequesterRepresentative>();
        system.register_trait_message::<MSG_EvaluationRequester_expect_n_results>();
        system.register_trait_message::<MSG_EvaluationRequester_on_result>();
    }

    pub fn register_implementor<Act: Actor + EvaluationRequester>(system: &mut ActorSystem) {
        system.register_implementor::<Act, EvaluationRequesterRepresentative>();
        system.add_handler::<Act, _, _>(
            |&MSG_EvaluationRequester_expect_n_results(resource, n), instance, world| {
                instance.expect_n_results(resource, n, world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_EvaluationRequester_on_result(ref result), instance, world| {
                instance.on_result(result, world); Fate::Live
            }, false
        );
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_EvaluationRequester_expect_n_results(pub Resource, pub u32);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_EvaluationRequester_on_result(pub EvaluatedSearchResult);

impl Actor for Market {
    type ID = MarketID;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Serialize, Deserialize)] #[serde(transparent)]
pub struct MarketID {
    _raw_id: RawID
}

impl Copy for MarketID {}
impl Clone for MarketID { fn clone(&self) -> Self { *self } }
impl ::std::fmt::Debug for MarketID {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "MarketID({:?})", self._raw_id)
    }
}
impl ::std::hash::Hash for MarketID {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self._raw_id.hash(state);
    }
}
impl PartialEq for MarketID {
    fn eq(&self, other: &MarketID) -> bool {
        self._raw_id == other._raw_id
    }
}
impl Eq for MarketID {}

impl TypedID for MarketID {
    type Target = Market;

    fn from_raw(id: RawID) -> Self {
        MarketID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl MarketID {
    pub fn spawn(world: &mut World) -> Self {
        let id = MarketID::from_raw(world.allocate_instance_id::<Market>());
        let swarm = world.local_broadcast::<Market>();
        world.send(swarm, MSG_Market_spawn(id, ));
        id
    }
    
    pub fn search(self, instant: Instant, location: RoughLocationID, resource: Resource, requester: EvaluationRequesterID, world: &mut World) {
        world.send(self.as_raw(), MSG_Market_search(instant, location, resource, requester));
    }
    
    pub fn register(self, resource: Resource, offer: OfferID, world: &mut World) {
        world.send(self.as_raw(), MSG_Market_register(resource, offer));
    }
    
    pub fn withdraw(self, resource: Resource, offer: OfferID, world: &mut World) {
        world.send(self.as_raw(), MSG_Market_withdraw(resource, offer));
    }
}

#[derive(Copy, Clone)] #[allow(non_camel_case_types)]
struct MSG_Market_spawn(pub MarketID, );
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Market_search(pub Instant, pub RoughLocationID, pub Resource, pub EvaluationRequesterID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Market_register(pub Resource, pub OfferID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Market_withdraw(pub Resource, pub OfferID);


impl Actor for TripCostEstimator {
    type ID = TripCostEstimatorID;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Serialize, Deserialize)] #[serde(transparent)]
pub struct TripCostEstimatorID {
    _raw_id: RawID
}

impl Copy for TripCostEstimatorID {}
impl Clone for TripCostEstimatorID { fn clone(&self) -> Self { *self } }
impl ::std::fmt::Debug for TripCostEstimatorID {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "TripCostEstimatorID({:?})", self._raw_id)
    }
}
impl ::std::hash::Hash for TripCostEstimatorID {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self._raw_id.hash(state);
    }
}
impl PartialEq for TripCostEstimatorID {
    fn eq(&self, other: &TripCostEstimatorID) -> bool {
        self._raw_id == other._raw_id
    }
}
impl Eq for TripCostEstimatorID {}

impl TypedID for TripCostEstimatorID {
    type Target = TripCostEstimator;

    fn from_raw(id: RawID) -> Self {
        TripCostEstimatorID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl TripCostEstimatorID {
    pub fn spawn(requester: EvaluationRequesterID, rough_source: RoughLocationID, rough_destination: RoughLocationID, base_result: EvaluatedSearchResult, instant: Instant, world: &mut World) -> Self {
        let id = TripCostEstimatorID::from_raw(world.allocate_instance_id::<TripCostEstimator>());
        let swarm = world.local_broadcast::<TripCostEstimator>();
        world.send(swarm, MSG_TripCostEstimator_spawn(id, requester, rough_source, rough_destination, base_result, instant));
        id
    }
    
    pub fn done(self, world: &mut World) {
        world.send(self.as_raw(), MSG_TripCostEstimator_done());
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_TripCostEstimator_spawn(pub TripCostEstimatorID, pub EvaluationRequesterID, pub RoughLocationID, pub RoughLocationID, pub EvaluatedSearchResult, pub Instant);
#[derive(Copy, Clone)] #[allow(non_camel_case_types)]
struct MSG_TripCostEstimator_done();

impl Into<LocationRequesterID> for TripCostEstimatorID {
    fn into(self) -> LocationRequesterID {
        LocationRequesterID::from_raw(self.as_raw())
    }
}

impl Into<DistanceRequesterID> for TripCostEstimatorID {
    fn into(self) -> DistanceRequesterID {
        DistanceRequesterID::from_raw(self.as_raw())
    }
}

#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    EvaluationRequesterID::register_trait(system);
    
    system.add_spawner::<Market, _, _>(
        |&MSG_Market_spawn(id, ), world| {
            Market::spawn(id, world)
        }, false
    );
    
    system.add_handler::<Market, _, _>(
        |&MSG_Market_search(instant, location, resource, requester), instance, world| {
            instance.search(instant, location, resource, requester, world); Fate::Live
        }, false
    );
    
    system.add_handler::<Market, _, _>(
        |&MSG_Market_register(resource, offer), instance, world| {
            instance.register(resource, offer, world); Fate::Live
        }, false
    );
    
    system.add_handler::<Market, _, _>(
        |&MSG_Market_withdraw(resource, offer), instance, world| {
            instance.withdraw(resource, offer, world); Fate::Live
        }, false
    );
    LocationRequesterID::register_implementor::<TripCostEstimator>(system);
    DistanceRequesterID::register_implementor::<TripCostEstimator>(system);
    system.add_spawner::<TripCostEstimator, _, _>(
        |&MSG_TripCostEstimator_spawn(id, requester, rough_source, rough_destination, ref base_result, instant), world| {
            TripCostEstimator::spawn(id, requester, rough_source, rough_destination, base_result, instant, world)
        }, false
    );
    
    system.add_handler::<TripCostEstimator, _, _>(
        |&MSG_TripCostEstimator_done(), instance, world| {
            instance.done(world)
        }, false
    );
}