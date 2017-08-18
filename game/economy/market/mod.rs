use kay::{ActorSystem, ID, Fate, World};
use compact::{CVec, CDict};
use super::resources::{ResourceMap, ResourceId, ResourceAmount};
use super::households::{HouseholdID, MemberIdx};
use core::simulation::{TimeOfDay, DurationSeconds, Timestamp};

#[derive(Compact, Clone)]
pub struct Deal {
    pub duration: DurationSeconds,
    pub take: ResourceMap<ResourceAmount>,
    pub give: (ResourceId, ResourceAmount),
}

#[derive(Compact, Clone)]
pub struct Offer {
    id: OfferID,
    by: ID,
    location: RoughDestinationID,
    from: TimeOfDay,
    to: TimeOfDay,
    deal: Deal,
    users: CVec<(HouseholdID, Option<MemberIdx>)>,
}

impl Offer {
    pub fn evaluate(
        &mut self,
        tick: Timestamp,
        location: RoughDestinationID,
        requester: EvaluationRequesterID,
        world: &mut World,
    ) {
        if TimeOfDay::from_tick(tick) < self.to {
            let search_result = EvaluatedSearchResult {
                resource: self.deal.give.0,
                n_to_expect: 1,
                some_results: vec![
                    EvaluatedDeal {
                        offer: self.id,
                        deal: self.deal.clone(),
                        from: self.from,
                        to: self.to,
                    },
                ].into(),
            };
            TripCostEstimatorID::spawn(
                requester,
                location,
                self.location,
                search_result,
                tick,
                world,
            );
        } else {
            requester.on_result(
                EvaluatedSearchResult {
                    resource: self.deal.give.0,
                    n_to_expect: 0,
                    some_results: CVec::new(),
                },
                world,
            );
        }
    }

    pub fn get_applicable_deal(
        &mut self,
        household: HouseholdID,
        member: MemberIdx,
        world: &mut World,
    ) {
        household.on_applicable_deal(self.deal.clone(), member, world);
    }

    pub fn started_using(
        &mut self,
        household: HouseholdID,
        member: Option<MemberIdx>,
        _: &mut World,
    ) {
        self.users.push((household, member));
    }

    pub fn stopped_using(
        &mut self,
        household: HouseholdID,
        member: Option<MemberIdx>,
        _: &mut World,
    ) {
        self.users.retain(|&(o_household, o_member)| {
            o_household != household || o_member != member
        });
    }
}

use transport::pathfinding::{RoughDestination, RoughDestinationID,
                             MSG_RoughDestination_query_as_destination, AsDestinationRequesterID,
                             MSG_AsDestinationRequester_tell_as_destination};

impl RoughDestination for Offer {
    fn query_as_destination(
        &mut self,
        requester: AsDestinationRequesterID,
        rough_destination: RoughDestinationID,
        tick: Timestamp,
        world: &mut World,
    ) {
        self.location.query_as_destination(
            requester,
            rough_destination,
            tick,
            world,
        );
    }
}

pub trait EvaluationRequester {
    fn on_result(&mut self, result: &EvaluatedSearchResult, world: &mut World);
}

pub struct Market {
    offers_by_resource: CDict<ResourceId, CVec<OfferID>>,
}

#[derive(Copy, Clone)]
pub struct Search {
    pub time: TimeOfDay,
    pub location: RoughDestinationID,
    pub resource: ResourceId,
    pub requester: ID,
}

#[derive(Compact, Clone)]
pub struct EvaluatedDeal {
    pub offer: OfferID,
    pub deal: Deal,
    pub from: TimeOfDay,
    pub to: TimeOfDay,
}

#[derive(Compact, Clone)]
pub struct EvaluatedSearchResult {
    pub resource: ResourceId,
    pub n_to_expect: usize,
    pub some_results: CVec<EvaluatedDeal>,
}

use transport::pathfinding::{Destination, AsDestinationRequester, GetDistanceTo,
                             DistanceRequester, DistanceRequesterID,
                             MSG_DistanceRequester_on_distance};

#[derive(Compact, Clone)]
pub struct TripCostEstimator {
    id: TripCostEstimatorID,
    requester: EvaluationRequesterID,
    rough_source: RoughDestinationID,
    source: Option<Destination>,
    rough_destination: RoughDestinationID,
    destination: Option<Destination>,
    base_result: EvaluatedSearchResult,
}

impl TripCostEstimator {
    pub fn spawn(
        id: TripCostEstimatorID,
        requester: EvaluationRequesterID,
        rough_source: RoughDestinationID,
        rough_destination: RoughDestinationID,
        base_result: &EvaluatedSearchResult,
        tick: Timestamp,
        world: &mut World,
    ) -> TripCostEstimator {
        rough_source.query_as_destination(id.into(), rough_source, tick, world);
        rough_destination.query_as_destination(id.into(), rough_destination, tick, world);
        TripCostEstimator {
            id,
            requester,
            rough_source,
            rough_destination,
            base_result: base_result.clone(),
            source: None,
            destination: None,
        }
    }
}

impl AsDestinationRequester for TripCostEstimator {
    fn tell_as_destination(
        &mut self,
        rough_destination: RoughDestinationID,
        as_destination: Option<Destination>,
        _tick: Timestamp,
        world: &mut World,
    ) {
        if self.rough_source == rough_destination {
            self.source = as_destination;
        } else if self.rough_destination == rough_destination {
            self.destination = as_destination;
        } else {
            panic!("Should have this rough source/destination")
        }

        if let (Some(source), Some(destination)) = (self.source, self.destination) {
            world.send(
                source.node,
                GetDistanceTo { destination, requester: self.id.into() },
            );
        }
    }
}

impl DistanceRequester for TripCostEstimator {
    fn on_distance(&mut self, maybe_distance: Option<f32>, world: &mut World) {
        const ASSUMED_AVG_SPEED: f32 = 10.0; // m/s

        let result = if let Some(distance) = maybe_distance {
            EvaluatedSearchResult {
                some_results: self.base_result
                    .some_results
                    .iter()
                    .map(|evaluated_deal| {
                        let estimated_travel_time =
                            DurationSeconds::new((distance / ASSUMED_AVG_SPEED) as usize);
                        let mut new_deal = evaluated_deal.clone();
                        new_deal.deal.duration += estimated_travel_time;
                        new_deal.from -= estimated_travel_time;
                        new_deal.to -= estimated_travel_time;
                        // TODO: adjust possible-until and resources
                        new_deal
                    })
                    .collect(),
                ..self.base_result
            }
        } else {
            EvaluatedSearchResult {
                resource: self.base_result.resource,
                n_to_expect: 0,
                some_results: CVec::new(),
            }
        };
        self.requester.on_result(result, world)
    }
}

pub fn setup(system: &mut ActorSystem) {
    kay_auto::auto_setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;
