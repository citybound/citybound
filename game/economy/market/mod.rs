use kay::{ActorSystem, Fate, World};
use compact::{CVec, CDict};
use super::resources::{Inventory, Entry, ResourceId, ResourceAmount};
use super::households::{HouseholdID, MemberIdx};
use core::simulation::{TimeOfDay, TimeOfDayRange, Duration, Instant};

#[derive(Compact, Clone)]
pub struct Deal {
    pub duration: Duration,
    pub delta: Inventory,
}

impl Deal {
    pub fn new<T: IntoIterator<Item = (ResourceId, ResourceAmount)>>(
        delta: T,
        duration: Duration,
    ) -> Self {
        Deal {
            duration,
            delta: delta.into_iter().collect(),
        }
    }

    pub fn main_given(&self) -> ResourceId {
        self.delta
            .iter()
            .filter_map(|&Entry(resource, amount)| if amount > 0.0 {
                Some(resource)
            } else {
                None
            })
            .next()
            .unwrap()
    }
}

#[derive(Compact, Clone)]
pub struct Offer {
    id: OfferID,
    offerer: HouseholdID,
    offering_member: MemberIdx,
    location: RoughLocationID,
    opening_hours: TimeOfDayRange,
    deal: Deal,
    users: CVec<(HouseholdID, Option<MemberIdx>)>,
    active_users: CVec<(HouseholdID, MemberIdx)>,
    being_withdrawn: bool,
}

impl Offer {
    pub fn register(
        id: OfferID,
        offerer: HouseholdID,
        offering_member: MemberIdx,
        location: RoughLocationID,
        opening_hours: TimeOfDayRange,
        deal: &Deal,
        world: &mut World,
    ) -> Offer {
        MarketID::global_first(world).register(deal.main_given(), id, world);

        Offer {
            id,
            offerer,
            offering_member,
            location,
            opening_hours,
            deal: deal.clone(),
            users: CVec::new(),
            active_users: CVec::new(),
            being_withdrawn: false,
        }
    }

    // create an internal offer, only known manually to members of one household
    pub fn internal(
        id: OfferID,
        offerer: HouseholdID,
        offering_member: MemberIdx,
        location: RoughLocationID,
        opening_hours: TimeOfDayRange,
        deal: &Deal,
        _: &mut World,
    ) -> Offer {
        Offer {
            id,
            offerer,
            offering_member,
            location,
            opening_hours,
            deal: deal.clone(),
            users: CVec::new(),
            active_users: CVec::new(),
            being_withdrawn: false,
        }
    }

    // The offer stays alive until the withdrawal is confirmed
    // to prevent offers being used while they're being withdrawn
    pub fn withdraw(&mut self, world: &mut World) {
        MarketID::global_first(world).withdraw(self.deal.main_given(), self.id, world);
        self.being_withdrawn = true;
    }

    // Internal users are manually responsible for forgetting about this offer
    pub fn withdraw_internal(&mut self, _: &mut World) -> Fate {
        Fate::Die
    }

    // TODO: there is still a tiny potential race condition here:
    //       1) household finds offer in market -> household
    //       2) offer withdrawn from market
    //       3) withdrawal confirmed
    //       ... starting to notify existing users
    //       4) household starts using offer
    //       => dangling single user keeping the offer half-dead
    pub fn withdrawal_confirmed(&mut self, world: &mut World) -> Fate {
        if self.users.is_empty() {

            Fate::Die
        } else {

            for user in &self.users {
                user.0.stop_using(self.id, world);
            }

            for &(active_user_household, active_member) in &self.active_users {
                active_user_household.reset_member_task(active_member, world);
            }

            Fate::Live // ...for now
        }
    }

    pub fn evaluate(
        &mut self,
        instant: Instant,
        location: RoughLocationID,
        requester: EvaluationRequesterID,
        world: &mut World,
    ) {
        if self.opening_hours.end_after_on_same_day(
            TimeOfDay::from(instant),
        )
        {
            let search_result = EvaluatedSearchResult {
                resource: self.deal.main_given(),
                evaluated_deals: vec![
                    EvaluatedDeal {
                        offer: self.id,
                        deal: self.deal.clone(),
                        opening_hours: self.opening_hours,
                    },
                ].into(),
            };
            TripCostEstimatorID::spawn(
                requester,
                location,
                self.location,
                search_result,
                instant,
                world,
            );
        } else {
            requester.on_result(
                EvaluatedSearchResult {
                    resource: self.deal.main_given(),
                    evaluated_deals: CVec::new(),
                },
                world,
            );
        }
    }

    pub fn request_receive_deal(
        &mut self,
        household: HouseholdID,
        member: MemberIdx,
        world: &mut World,
    ) {
        self.offerer.provide_deal(
            self.deal.clone(),
            self.offering_member,
            world,
        );
        household.receive_deal(self.deal.clone(), member, world);
    }

    pub fn request_receive_undo_deal(
        &mut self,
        household: HouseholdID,
        member: MemberIdx,
        world: &mut World,
    ) {
        self.offerer.receive_deal(
            self.deal.clone(),
            self.offering_member,
            world,
        );
        household.provide_deal(self.deal.clone(), member, world);
    }

    pub fn started_using(
        &mut self,
        household: HouseholdID,
        member: Option<MemberIdx>,
        _: &mut World,
    ) {
        if !self.users.contains(&(household, member)) {
            self.users.push((household, member));
        }
    }

    pub fn stopped_using(
        &mut self,
        household: HouseholdID,
        member: Option<MemberIdx>,
        _: &mut World,
    ) -> Fate {
        self.users.retain(|&(o_household, o_member)| {
            o_household != household || o_member != member
        });

        if self.users.is_empty() && self.being_withdrawn {

            Fate::Die
        } else {
            Fate::Live
        }
    }

    pub fn started_actively_using(
        &mut self,
        household: HouseholdID,
        member: MemberIdx,
        _: &mut World,
    ) {
        if !self.active_users.contains(&(household, member)) {
            self.active_users.push((household, member));
        }
    }

    pub fn stopped_actively_using(
        &mut self,
        household: HouseholdID,
        member: MemberIdx,
        _: &mut World,
    ) {
        self.active_users.retain(|&(o_household, o_member)| {
            o_household != household || o_member != member
        });
    }
}

use transport::pathfinding::{RoughLocation, RoughLocationID,
                             MSG_RoughLocation_resolve_as_location,
                             MSG_RoughLocation_resolve_as_position, LocationRequesterID,
                             PositionRequesterID, MSG_LocationRequester_location_resolved};

impl RoughLocation for Offer {
    fn resolve_as_location(
        &mut self,
        requester: LocationRequesterID,
        rough_location: RoughLocationID,
        instant: Instant,
        world: &mut World,
    ) {
        self.location.resolve_as_location(
            requester,
            rough_location,
            instant,
            world,
        );
    }

    fn resolve_as_position(
        &mut self,
        requester: PositionRequesterID,
        rough_location: RoughLocationID,
        world: &mut World,
    ) {
        self.location.resolve_as_position(
            requester,
            rough_location,
            world,
        );
    }
}

pub trait EvaluationRequester {
    fn expect_n_results(&mut self, resource: ResourceId, n: usize, world: &mut World);
    fn on_result(&mut self, result: &EvaluatedSearchResult, world: &mut World);
}

#[derive(Compact, Clone)]
pub struct Market {
    id: MarketID,
    offers_by_resource: CDict<ResourceId, CVec<OfferID>>,
}

impl Market {
    pub fn spawn(id: MarketID, _: &mut World) -> Market {
        Market { id, offers_by_resource: CDict::new() }
    }

    pub fn search(
        &mut self,
        instant: Instant,
        location: RoughLocationID,
        resource: ResourceId,
        requester: EvaluationRequesterID,
        world: &mut World,
    ) {
        let n_to_expect = if let Some(offers) = self.offers_by_resource.get(resource) {
            for offer in offers.iter() {
                offer.evaluate(instant, location, requester, world);
            }

            offers.len()
        } else {
            0
        };

        requester.expect_n_results(resource, n_to_expect, world);
    }

    pub fn register(&mut self, resource: ResourceId, offer: OfferID, _: &mut World) {
        self.offers_by_resource.push_at(resource, offer);
    }

    pub fn withdraw(&mut self, resource: ResourceId, offer: OfferID, world: &mut World) {
        if let Some(offers) = self.offers_by_resource.get_mut(resource) {
            offers.retain(|o| *o != offer);
        }
        offer.withdrawal_confirmed(world);
    }
}

#[derive(Compact, Clone)]
pub struct EvaluatedDeal {
    pub offer: OfferID,
    pub deal: Deal,
    pub opening_hours: TimeOfDayRange,
}

#[derive(Compact, Clone)]
pub struct EvaluatedSearchResult {
    pub resource: ResourceId,
    pub evaluated_deals: CVec<EvaluatedDeal>,
}

use transport::pathfinding::{Location, LocationRequester, DistanceRequester, DistanceRequesterID,
                             MSG_DistanceRequester_on_distance};

#[derive(Compact, Clone)]
pub struct TripCostEstimator {
    id: TripCostEstimatorID,
    requester: EvaluationRequesterID,
    rough_source: RoughLocationID,
    source: Option<Location>,
    rough_destination: RoughLocationID,
    destination: Option<Location>,
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
        location: Option<Location>,
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
            source.node.get_distance_to(
                destination,
                self.id.into(),
                world,
            );
        } else if self.n_resolved == 2 {
            // println!(
            //     "Either source or dest not resolvable for {}",
            //     r_info(self.base_result.resource).0
            // );
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
                evaluated_deals: self.base_result
                    .evaluated_deals
                    .iter()
                    .map(|evaluated_deal| {
                        let estimated_travel_time =
                            Duration((distance / ASSUMED_AVG_SPEED) as usize);
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
    system.register::<Offer>();
    system.register::<Market>();
    system.register::<TripCostEstimator>();

    kay_auto::auto_setup(system);

    if system.networking_machine_id() == 0 {
        MarketID::spawn(&mut system.world());
    }
}

mod kay_auto;
pub use self::kay_auto::*;
