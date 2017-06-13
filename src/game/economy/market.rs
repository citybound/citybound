use kay::{ActorSystem, ID, Fate};
use kay::swarm::{Swarm, SubActor, CreateWith};
use compact::{CVec, CDict};
use super::resources::{ResourceMap, ResourceId, ResourceAmount};
use super::households::MemberIdx;
use core::simulation::{TimeOfDay, DurationSeconds};

#[derive(Compact, Clone)]
pub struct Deal {
    pub duration: DurationSeconds,
    pub take: ResourceMap<ResourceAmount>,
    pub give: (ResourceId, ResourceAmount),
}

#[derive(Compact, Clone, SubActor)]
pub struct Offer {
    _id: Option<ID>,
    by: ID,
    location: ID, // lane
    from: TimeOfDay,
    to: TimeOfDay,
    deal: Deal,
    users: CVec<(ID, Option<MemberIdx>)>,
}

pub struct Market {
    offers_by_resource: CDict<ResourceId, CVec<ID>>,
}

#[derive(Copy, Clone)]
pub struct Evaluate {
    pub time: TimeOfDay,
    pub location: ID,
    pub requester: ID,
}

#[derive(Copy, Clone)]
pub struct Search {
    pub time: TimeOfDay,
    pub location: ID,
    pub resource: ResourceId,
    pub requester: ID,
}

#[derive(Compact, Clone)]
pub struct EvaluatedDeal {
    pub offer: ID,
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

#[derive(Copy, Clone)]
pub struct GetApplicableDeal(pub ID, pub MemberIdx);

#[derive(Compact, Clone)]
pub struct ApplicableDeal(pub Deal, pub MemberIdx);


#[derive(Copy, Clone)]
pub struct StartedUsing(pub ID, pub Option<MemberIdx>);

#[derive(Copy, Clone)]
pub struct StoppedUsing(pub ID, pub Option<MemberIdx>);

use game::lanes_and_cars::pathfinding::{Destination, QueryAsDestination, TellAsDestination,
                                        GetDistanceTo, DistanceInfo};

#[derive(SubActor, Compact, Clone)]
pub struct TripCostEstimator {
    _id: Option<ID>,
    requester: ID,
    rough_source: ID,
    source: Option<Destination>,
    rough_destination: ID,
    destination: Option<Destination>,
    base_result: EvaluatedSearchResult,
}

impl TripCostEstimator {
    pub fn new(requester: ID,
               rough_source: ID,
               rough_destination: ID,
               base_result: EvaluatedSearchResult)
               -> Self {
        TripCostEstimator {
            _id: None,
            requester,
            rough_source,
            rough_destination,
            base_result,
            source: None,
            destination: None,
        }
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.add(
        Swarm::<Offer>::new(),
        Swarm::<Offer>::subactors(|mut each_offer| {

            each_offer.on(|&Evaluate { time, location, requester }, offer, world| {
                if time < offer.to {
                    let search_result = EvaluatedSearchResult {
                        resource: offer.deal.give.0,
                        n_to_expect: 1,
                        some_results: vec![EvaluatedDeal {
                                               offer: offer.id(),
                                               deal: offer.deal.clone(),
                                               from: offer.from,
                                               to: offer.to,
                                           }].into(),
                    };
                    world.send_to_id_of::<Swarm<TripCostEstimator>, _>(CreateWith(
                        TripCostEstimator::new(requester, location, offer.location, search_result), ()))
                } else {
                    world.send(requester,
                               EvaluatedSearchResult {
                                   resource: offer.deal.give.0,
                                   n_to_expect: 0,
                                   some_results: CVec::new(),
                               });
                }
                Fate::Live
            });

            each_offer.on(|&GetApplicableDeal(id, member), offer, world| {
                world.send(id, ApplicableDeal(offer.deal.clone(), member));
                Fate::Live
            });

            each_offer.on(|&StartedUsing(id, member), offer, _| {
                offer.users.push((id, member));
                Fate::Live
            });

            each_offer.on(|&StoppedUsing(id, member), offer, _| {
                offer
                    .users
                    .retain(|&(o_id, o_member)| o_id != id || o_member != member);
                Fate::Live
            })
        }),
    );


    system.add(
        Swarm::<TripCostEstimator>::new(),
        Swarm::<TripCostEstimator>::subactors(|mut each_estimator| {

            each_estimator.on_create_with(|_: &(), estimator, world| {
                world.send(estimator.rough_source,
                           QueryAsDestination {
                               requester: estimator.id(),
                               rough_destination: estimator.rough_source,
                               tick: None,
                           });

                world.send(estimator.rough_destination,
                           QueryAsDestination {
                               requester: estimator.id(),
                               rough_destination: estimator.rough_destination,
                               tick: None,
                           });

                Fate::Live
            });

            each_estimator.on(|&TellAsDestination {
                 rough_destination,
                 as_destination,
                 ..
             },
             estimator,
             world| {
                if estimator.rough_source == rough_destination {
                    estimator.source = as_destination;
                } else if estimator.rough_destination == rough_destination {
                    estimator.destination = as_destination;
                } else {
                    panic!("Should have this rough source/destination")
                }

                if let (Some(source), Some(destination)) =
                    (estimator.source, estimator.destination) {
                    world.send(source.node,
                               GetDistanceTo { destination, requester: estimator.id() });
                }
                Fate::Live
            });

            const ASSUMED_AVG_SPEED: f32 = 10.0; // m/s

            each_estimator.on(|&DistanceInfo(maybe_distance), estimator, world| {
                let result = if let Some(distance) = maybe_distance {
                    EvaluatedSearchResult {
                        some_results: estimator
                            .base_result
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
                        ..estimator.base_result
                    }
                } else {
                    EvaluatedSearchResult {
                        resource: estimator.base_result.resource,
                        n_to_expect: 0,
                        some_results: CVec::new(),
                    }
                };
                world.send(estimator.requester, result);
                Fate::Die
            });
        }),
    );
}
