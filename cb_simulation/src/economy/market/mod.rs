use kay::{ActorSystem, Fate, World, Actor};
use compact::{CVec, CDict};
use super::resources::{Inventory, Entry, Resource, ResourceAmount};
use super::households::OfferID;
use time::{TimeOfDayRange, Duration, Instant};
use transport::pathfinding::{RoughLocationID, LocationRequesterID};

#[derive(Compact, Clone, Serialize, Deserialize)]
pub struct Deal {
    pub duration: Duration,
    pub delta: Inventory,
}

impl Deal {
    pub fn new<T: IntoIterator<Item = (Resource, ResourceAmount)>>(
        delta: T,
        duration: Duration,
    ) -> Self {
        Deal {
            duration,
            delta: delta.into_iter().collect(),
        }
    }

    pub fn main_given(&self) -> Resource {
        self.delta
            .iter()
            .filter_map(|&Entry(resource, amount)| if amount > 0.0 { Some(resource) } else { None })
            .next()
            .unwrap()
    }
}

pub trait EvaluationRequester {
    fn expect_n_results(&mut self, resource: Resource, n: u32, world: &mut World);
    fn on_result(&mut self, result: &EvaluatedSearchResult, world: &mut World);
}

#[derive(Compact, Clone)]
pub struct Market {
    id: MarketID,
    offers_by_resource: CDict<Resource, CVec<OfferID>>,
}

impl Market {
    pub fn spawn(id: MarketID, _: &mut World) -> Market {
        Market {
            id,
            offers_by_resource: CDict::new(),
        }
    }

    pub fn search(
        &mut self,
        instant: Instant,
        location: RoughLocationID,
        resource: Resource,
        requester: EvaluationRequesterID,
        world: &mut World,
    ) {
        let n_to_expect = if let Some(offers) = self.offers_by_resource.get(resource) {
            for offer in offers.iter() {
                offer
                    .household
                    .evaluate(offer.idx, instant, location, requester, world);
            }

            offers.len()
        } else {
            0
        };

        requester.expect_n_results(resource, n_to_expect as u32, world);
    }

    pub fn register(&mut self, resource: Resource, offer: OfferID, _: &mut World) {
        self.offers_by_resource.push_at(resource, offer);
    }

    pub fn withdraw(&mut self, resource: Resource, offer: OfferID, world: &mut World) {
        if let Some(offers) = self.offers_by_resource.get_mut(resource) {
            offers.retain(|o| *o != offer);
        }
        offer.household.withdrawal_confirmed(offer.idx, world);
    }
}

#[derive(Compact, Clone, Serialize, Deserialize)]
pub struct EvaluatedDeal {
    pub offer: OfferID,
    pub deal: Deal,
    pub opening_hours: TimeOfDayRange,
}

#[derive(Compact, Clone)]
pub struct EvaluatedSearchResult {
    pub resource: Resource,
    pub evaluated_deals: CVec<EvaluatedDeal>,
}

use transport::pathfinding::{PreciseLocation, LocationRequester, DistanceRequester,
DistanceRequesterID};

#[derive(Compact, Clone)]
pub struct TripCostEstimator {
    id: TripCostEstimatorID,
    requester: EvaluationRequesterID,
    rough_source: RoughLocationID,
    source: Option<PreciseLocation>,
    rough_destination: RoughLocationID,
    destination: Option<PreciseLocation>,
    n_resolved: u8,
    base_result: EvaluatedSearchResult,
}

impl TripCostEstimator {
    pub fn spawn(
        id: TripCostEstimatorID,
        requester: EvaluationRequesterID,
        rough_source: RoughLocationID,
        rough_destination: RoughLocationID,
        base_result: &EvaluatedSearchResult,
        instant: Instant,
        world: &mut World,
    ) -> TripCostEstimator {
        rough_source.resolve_as_location(id.into(), rough_source, instant, world);
        rough_destination.resolve_as_location(id.into(), rough_destination, instant, world);

        TripCostEstimator {
            id,
            requester,
            rough_source,
            rough_destination,
            base_result: base_result.clone(),
            source: None,
            n_resolved: 0,
            destination: None,
        }
    }

    pub fn done(&mut self, _: &mut World) -> Fate {
        Fate::Die
    }
}

impl LocationRequester for TripCostEstimator {
    fn location_resolved(
        &mut self,
        rough_location: RoughLocationID,
        location: Option<PreciseLocation>,
        _tick: Instant,
        world: &mut World,
    ) {
        if self.rough_source == rough_location {
            self.source = location;
        } else if self.rough_destination == rough_location {
            self.destination = location;
        } else {
            panic!("Should have this rough source/destination")
        }

        self.n_resolved += 1;

        if let (Some(source), Some(destination)) = (self.source, self.destination) {
            source
                .node
                .get_distance_to(destination.location, self.id_as(), world);
        } else if self.n_resolved == 2 {
            println!(
                "Either source or dest not resolvable for {}",
                self.base_result.resource
            );
            self.requester.on_result(
                EvaluatedSearchResult {
                    resource: self.base_result.resource,
                    evaluated_deals: CVec::new(),
                },
                world,
            );
            self.id.done(world);
        }
    }
}

impl DistanceRequester for TripCostEstimator {
    fn on_distance(&mut self, maybe_distance: Option<f32>, world: &mut World) {
        const ASSUMED_AVG_SPEED: f32 = 10.0; // m/s

        let result = if let Some(distance) = maybe_distance {
            EvaluatedSearchResult {
                evaluated_deals: self
                    .base_result
                    .evaluated_deals
                    .iter()
                    .map(|evaluated_deal| {
                        let estimated_travel_time = Duration((distance / ASSUMED_AVG_SPEED) as u32);
                        let mut new_deal = evaluated_deal.clone();
                        new_deal.deal.duration += estimated_travel_time;
                        new_deal.opening_hours =
                            new_deal.opening_hours.earlier_by(estimated_travel_time);
                        // TODO: adjust resources to incorporate travel costs
                        new_deal
                    })
                    .collect(),
                ..self.base_result
            }
        } else {
            // println!(
            //     "No distance for {}, from {:?} to {:?}",
            //     r_info(self.base_result.resource).0,
            //     self.source,
            //     self.destination
            // );
            EvaluatedSearchResult {
                resource: self.base_result.resource,
                evaluated_deals: CVec::new(),
            }
        };
        self.requester.on_result(result, world);
        self.id.done(world);
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.register::<Market>();
    system.register::<TripCostEstimator>();
    kay_auto::auto_setup(system);
}

pub fn spawn(world: &mut World) {
    MarketID::spawn(world);
}

mod kay_auto;
pub use self::kay_auto::*;
