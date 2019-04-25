use kay::{ActorSystem, World, Actor, TypedID, Fate};
use compact::{CVec, CDict, COption};
use time::{Duration, TimeOfDay, Instant, Ticks, TimeID, TICKS_PER_SIM_SECOND, Sleeper,
Temporal};
use cb_util::async_counter::AsyncCounter;
use cb_util::random::{seed, Rng};
use ordered_float::OrderedFloat;
use log::{debug, info, warn};
const LOG_T: &str = "Households";

pub mod tasks;
pub mod offers;
pub mod ui;

pub mod household_kinds;
use self::household_kinds::*;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct MemberIdx(u32);

impl MemberIdx {
    pub fn new(idx: usize) -> Self {
        MemberIdx(idx as u32)
    }

    pub fn as_idx(self) -> usize {
        self.0 as usize
    }
}

use super::market::{MarketID, Deal, EvaluatedDeal, EvaluationRequester, EvaluationRequesterID,
TripCostEstimatorID, EvaluatedSearchResult};
use super::resources::{Resource, ResourceAmount, ResourceMap, Entry, Inventory};
use transport::pathfinding::{RoughLocationID, RoughLocation};
use transport::pathfinding::trip::{TripListener, TripID, TripResult, TripFate};
use self::tasks::{Task, TaskState, TaskEndSchedulerID};
pub use self::offers::{Offer, OfferIdx, OfferID};

const N_TOP_PROBLEMS: usize = 5;
const DECISION_PAUSE: Ticks = Ticks(200);
const UPDATE_EVERY_N_SECS: u32 = 4;

// TODO: make kay_codegen figure this out on it's own
impl Into<RoughLocationID> for HouseholdID {
    fn into(self) -> RoughLocationID {
        RoughLocationID::from_raw(self.as_raw())
    }
}

pub trait Household:
    Actor + EvaluationRequester + Sleeper + Temporal + TripListener + RoughLocation
{
    fn core(&self) -> &HouseholdCore;
    fn core_mut(&mut self) -> &mut HouseholdCore;
    fn site(&self) -> RoughLocationID;

    fn is_shared(resource: Resource) -> bool;
    fn supplier_shared(resource: Resource) -> bool;
    fn importance(resource: Resource, time: TimeOfDay) -> f32;
    fn graveness(resource: Resource, amount: ResourceAmount, time: TimeOfDay) -> f32 {
        -amount * Self::importance(resource, time)
    }
    fn interesting_resources() -> &'static [Resource];
    fn decay(&mut self, dt: Duration, world: &mut World);

    fn household_name(&self) -> String;
    fn member_name(&self, member: MemberIdx) -> String;

    fn receive_deal(&mut self, deal: &Deal, member: MemberIdx, _: &mut World) {
        let core = self.core_mut();
        deal.delta.give_to_shared_private(
            &mut core.resources,
            &mut core.member_resources[member.as_idx()],
            Self::is_shared,
        );
    }

    fn provide_deal(&mut self, deal: &Deal, member: MemberIdx, _: &mut World) {
        let core = self.core_mut();
        let provide_wakefulness =
            deal.delta.len() == 1 && deal.delta.get(Resource::Wakefulness).is_some();
        if !provide_wakefulness {
            deal.delta.take_from_shared_private(
                &mut core.resources,
                &mut core.member_resources[member.as_idx()],
                Self::is_shared,
            );
        }
    }

    fn task_succeeded(&mut self, member: MemberIdx, world: &mut World) {
        {
            debug(
                LOG_T,
                format!("Member {} task succeeded", member.0),
                self.id(),
                world,
            );
            if let TaskState::StartedAt(_, location) =
                self.core().member_tasks[member.as_idx()].state
            {
                self.stop_task(member, Some(location), world);
            } else {
                panic!("Can't finish unstarted task");
            }
        }
    }

    fn task_failed(&mut self, member: MemberIdx, location: RoughLocationID, world: &mut World) {
        self.stop_task(member, Some(location), world);
    }

    fn reset_member_task(&mut self, member: MemberIdx, world: &mut World) {
        debug(
            LOG_T,
            format!("Reset member {}", member.0),
            self.id(),
            world,
        );
        TaskEndSchedulerID::local_first(world).deschedule(self.id_as(), member, world);

        self.stop_task(member, None, world);
    }

    fn stop_using(&mut self, offer: OfferID, world: &mut World) {
        if let Some(Entry(associated_resource, _)) = self
            .core()
            .used_offers
            .iter()
            .find(|&&Entry(_, resource_offer)| resource_offer == offer)
            .cloned()
        {
            self.core_mut().used_offers.remove(associated_resource);
            offer
                .household
                .stopped_using(offer.idx, self.id_as(), None, world);
        }

        let id_as_household = self.id_as();

        for (i, member_used_offers) in self.core_mut().member_used_offers.iter_mut().enumerate() {
            if let Some(Entry(associated_resource, _)) = member_used_offers
                .iter()
                .find(|&&Entry(_, resource_offer)| resource_offer == offer)
                .cloned()
            {
                member_used_offers.remove(associated_resource);
                offer.household.stopped_using(
                    offer.idx,
                    id_as_household,
                    Some(MemberIdx::new(i)),
                    world,
                );
            }
        }

        let members_to_reset = self
            .core()
            .member_tasks
            .iter()
            .enumerate()
            .filter_map(|(i, task)| {
                if let Task {
                    goal: Some((_, task_offer)),
                    ..
                } = *task
                {
                    if task_offer == offer {
                        Some(MemberIdx::new(i))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        for member_to_reset in members_to_reset {
            self.reset_member_task(member_to_reset, world);
        }
    }

    fn destroy(&mut self, world: &mut World) {
        self.core_mut().being_destroyed = true;

        for &Entry(_, offer) in self.core().used_offers.iter() {
            offer
                .household
                .stopped_using(offer.idx, self.id_as(), None, world);
        }

        for (i, member_used_offers) in self.core().member_used_offers.iter().enumerate() {
            for &Entry(_, offer) in member_used_offers.iter() {
                offer.household.stopped_using(
                    offer.idx,
                    self.id_as(),
                    Some(MemberIdx::new(i)),
                    world,
                );
            }
        }

        for (idx, offer) in self.core().provided_offers.iter().enumerate() {
            MarketID::local_first(world).withdraw(
                offer.deal.main_given(),
                OfferID {
                    household: self.id_as(),
                    idx: OfferIdx(idx as u16),
                },
                world,
            )
        }

        self.on_destroy(world);
    }
    fn on_destroy(&mut self, world: &mut World);

    fn update_core(&mut self, current_instant: Instant, world: &mut World) {
        if let DecisionState::None = self.core().decision_state {
            let idle_members_idx_loc = self
                .core()
                .member_tasks
                .iter()
                .enumerate()
                .filter_map(|(idx, m)| match m.state {
                    TaskState::IdleAt(loc) => Some((idx, loc)),
                    _ => None,
                })
                .collect::<Vec<_>>();
            let mut rng = seed((current_instant.ticks(), self.id()));
            let maybe_idle_idx_loc = rng.choose(&idle_members_idx_loc);
            if let Some(&(idle_member_idx, location)) = maybe_idle_idx_loc {
                self.find_new_task_for(
                    MemberIdx::new(idle_member_idx),
                    current_instant,
                    location,
                    world,
                );
            }
        };
    }

    fn top_problems(&self, member: MemberIdx, time: TimeOfDay) -> Vec<(Resource, f32)> {
        let mut resource_graveness = self
            .core()
            .resources
            .iter()
            .chain(self.core().member_resources[member.as_idx()].iter())
            .filter_map(|&Entry(resource, amount)| {
                let graveness = Self::graveness(resource, amount, time);
                if graveness > 0.1 {
                    Some((resource, graveness))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        resource_graveness.sort_by_key(|&(_r, i)| OrderedFloat(i));

        resource_graveness.truncate(N_TOP_PROBLEMS);
        resource_graveness
    }

    fn find_new_task_for(
        &mut self,
        member: MemberIdx,
        instant: Instant,
        location: RoughLocationID,
        world: &mut World,
    ) {
        debug(LOG_T, "Top N Problems", self.id(), world);

        let time = TimeOfDay::from(instant);
        let top_problems = self.top_problems(member, time);

        if top_problems.is_empty() {
            TimeID::local_first(world).wake_up_in(DECISION_PAUSE, self.id_as(), world);
        } else {
            let mut decision_entries = CDict::<Resource, DecisionResourceEntry>::new();
            let id_as_eval_requester = self.id_as();
            let log_as = self.id();
            let core = self.core_mut();

            for &(resource, graveness) in &top_problems {
                debug(
                    LOG_T,
                    format!("Member #{}: {} = {}", member.as_idx(), resource, graveness),
                    log_as,
                    world,
                );
                let maybe_offer = if Self::supplier_shared(resource) {
                    core.used_offers.get(resource)
                } else {
                    core.member_used_offers[member.as_idx()].get(resource)
                };

                let initial_counter = if let Some(&offer) = maybe_offer {
                    debug(
                        LOG_T,
                        format!(" -> Using favorite offer {:?} for {}\n", offer, resource),
                        log_as,
                        world,
                    );
                    offer.household.evaluate(
                        offer.idx,
                        instant,
                        location,
                        id_as_eval_requester,
                        world,
                    );

                    AsyncCounter::with_target(1)
                } else {
                    debug(
                        LOG_T,
                        format!(" -> Doing market query for {}\n", resource),
                        log_as,
                        world,
                    );
                    MarketID::global_first(world).search(
                        instant,
                        location,
                        resource,
                        id_as_eval_requester,
                        world,
                    );

                    AsyncCounter::new()
                };

                decision_entries.insert(
                    resource,
                    DecisionResourceEntry {
                        results_counter: initial_counter,
                        best_deal: COption(None),
                        best_deal_usefulness: 0.0,
                    },
                );
            }

            core.decision_state =
                DecisionState::Choosing(member, instant, top_problems.into(), decision_entries);
        }
    }

    fn update_results(&mut self, resource: Resource, update: &ResultAspect, world: &mut World) {
        let done = {
            let log_as = self.id();
            let core = self.core_mut();

            if let DecisionState::Choosing(_, instant, ref top_problems, ref mut entries) =
                core.decision_state
            {
                {
                    let entry = entries
                        .get_mut(resource)
                        .expect("Should have an entry for queried resource");

                    match *update {
                        ResultAspect::AddDeals(ref evaluated_deals) => {
                            for evaluated_deal in evaluated_deals {
                                debug(
                                    LOG_T,
                                    format!(
                                        "Got eval'd deal for {}, {:?} -> {:?}\n",
                                        evaluated_deal.deal.main_given(),
                                        evaluated_deal.opening_hours.start.hours_minutes(),
                                        evaluated_deal.opening_hours.end.hours_minutes(),
                                    ),
                                    log_as,
                                    world,
                                );
                                if evaluated_deal.opening_hours.contains(instant) {
                                    let new_deal_usefulness = Self::deal_usefulness(
                                        top_problems,
                                        evaluated_deal,
                                        log_as,
                                        world,
                                    );
                                    if new_deal_usefulness > entry.best_deal_usefulness {
                                        entry.best_deal = COption(Some(evaluated_deal.clone()));
                                        entry.best_deal_usefulness = new_deal_usefulness;
                                    } else {
                                        debug(
                                            LOG_T,
                                            format!(
                                                "Deal rejected, not more useful: {} vs {}\n",
                                                new_deal_usefulness, entry.best_deal_usefulness
                                            ),
                                            log_as,
                                            world,
                                        );
                                    }
                                } else {
                                    debug(LOG_T, "Deal rejected: not open", log_as, world);
                                }
                            }

                            entry.results_counter.increment();
                        }
                        ResultAspect::SetTarget(n) => {
                            entry.results_counter.set_target(n as usize);
                        }
                    }
                }

                entries
                    .values()
                    .all(|entry| entry.results_counter.is_done())
            } else {
                warn(
                    LOG_T,
                    "Received unexpected deal / should be choosing",
                    log_as,
                    world,
                );
                false
            }
        };

        if done {
            self.choose_deal(world);
        }
    }

    fn deal_usefulness(
        top_problems: &[(Resource, f32)],
        evaluated: &EvaluatedDeal,
        logging_from: <Self as Actor>::ID,
        world: &mut World,
    ) -> f32 {
        let resource_graveness_improvement: f32 = top_problems
            .iter()
            .map(|&(resource, graveness)| {
                let delta = evaluated.deal.delta.get(resource).cloned().unwrap_or(0.0);
                let improvement_strength = delta * graveness;
                debug(
                    LOG_T,
                    format!(
                        "{} improves by {} (graveness {}, delta: {:?})\n",
                        resource,
                        improvement_strength,
                        graveness,
                        evaluated.deal.delta.get(resource)
                    )
                    .as_str(),
                    logging_from,
                    world,
                );
                improvement_strength
            })
            .sum();

        resource_graveness_improvement / evaluated.deal.duration.as_seconds()
    }

    fn choose_deal(&mut self, world: &mut World) {
        let log_as = self.id();
        let id_as_household = self.id_as();
        let id_as_sleeper = self.id_as();
        debug(LOG_T, "Choosing deal!", self.id(), world);

        let maybe_best_info = {
            let core = self.core_mut();

            if let DecisionState::Choosing(member, instant, _, ref entries) = core.decision_state {
                let maybe_best = most_useful_evaluated_deal(entries);

                if let Some(best) = maybe_best {
                    let task = &mut core.member_tasks[member.as_idx()];

                    *task = if let TaskState::IdleAt(location) = task.state {
                        Task {
                            goal: Some((best.deal.main_given(), best.offer)),
                            duration: best.deal.duration,
                            state: TaskState::GettingReadyAt(location),
                        }
                    } else {
                        panic!("Member who gets new task should be idle");
                    };

                    debug(
                        LOG_T,
                        format!("Found best offer for {}\n", best.deal.main_given()),
                        log_as,
                        world,
                    );

                    Some((member, instant, best))
                } else {
                    None
                }
            } else {
                panic!("Tried to choose deal while not deciding");
            }
        };

        if let Some((member, instant, best)) = maybe_best_info {
            {
                let (used_offers, maybe_member) = if Self::supplier_shared(best.deal.main_given()) {
                    (&mut self.core_mut().used_offers, None)
                } else {
                    (
                        &mut self.core_mut().member_used_offers[member.as_idx()],
                        Some(member),
                    )
                };
                if let Some(previous_offer) = used_offers.insert(best.deal.main_given(), best.offer)
                {
                    if previous_offer != best.offer {
                        previous_offer.household.stopped_using(
                            previous_offer.idx,
                            id_as_household,
                            maybe_member,
                            world,
                        );
                    }
                }
                best.offer.household.started_using(
                    best.offer.idx,
                    id_as_household,
                    maybe_member,
                    world,
                );
            }

            self.core_mut().decision_state = DecisionState::WaitingForTrip(member);
            best.offer.household.request_receive_deal(
                best.offer.idx,
                id_as_household,
                member,
                world,
            );
            self.start_trip(member, instant, world);
        } else {
            debug(
                LOG_T,
                "Didn't find any suitable offers at all",
                self.id(),
                world,
            );
            self.core_mut().decision_state = DecisionState::None;
            TimeID::local_first(world).wake_up_in(DECISION_PAUSE, id_as_sleeper, world);
        }

        fn most_useful_evaluated_deal(
            entries: &CDict<Resource, DecisionResourceEntry>,
        ) -> Option<EvaluatedDeal> {
            entries
                .values()
                .max_by_key(|decision_entry| OrderedFloat(decision_entry.best_deal_usefulness))
                .and_then(|best_entry| best_entry.best_deal.as_ref().cloned())
        }
    }

    fn start_trip(&mut self, member: MemberIdx, instant: Instant, world: &mut World) {
        if let Task {
            goal: Some((_, offer)),
            state: TaskState::GettingReadyAt(source),
            ..
        } = self.core().member_tasks[member.as_idx()]
        {
            TripID::spawn(
                source,
                offer.household.into(),
                Some(self.id_as()),
                instant,
                world,
            );
        } else {
            panic!("Member should be getting ready before starting trip");
        }
    }

    fn on_trip_created(&mut self, trip: TripID, world: &mut World) {
        self.core_mut().decision_state =
            if let DecisionState::WaitingForTrip(member) = self.core().decision_state {
                self.core_mut().member_tasks[member.as_idx()].state = TaskState::InTrip(trip);
                TimeID::local_first(world).wake_up_in(DECISION_PAUSE, self.id_as(), world);
                DecisionState::None
            } else {
                panic!("Should be in waiting for trip state")
            };
    }

    fn on_trip_result(
        &mut self,
        trip: TripID,
        result: TripResult,
        rough_source: RoughLocationID,
        rough_destination: RoughLocationID,
        world: &mut World,
    ) {
        let (matching_task_member, matching_resource, matching_offer) = self
            .core()
            .member_tasks
            .iter()
            .enumerate()
            .filter_map(|(idx, task)| {
                if let TaskState::InTrip(task_trip_id) = task.state {
                    if task_trip_id == trip {
                        if let Some((goal, offer)) = task.goal {
                            Some((MemberIdx::new(idx), goal, offer))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .next()
            .expect("Should have a matching task");
        {
            let id_as_household = self.id_as();

            let (used_offers, maybe_member) = if Self::supplier_shared(matching_resource) {
                (&mut self.core_mut().used_offers, None)
            } else {
                (
                    &mut self.core_mut().member_used_offers[matching_task_member.as_idx()],
                    Some(matching_task_member),
                )
            };

            match result.fate {
                TripFate::Success(_) => {}
                _ => {
                    used_offers.remove(matching_resource);
                    matching_offer.household.stopped_using(
                        matching_offer.idx,
                        id_as_household,
                        maybe_member,
                        world,
                    );
                }
            }
        }

        match result.fate {
            TripFate::Success(instant) => {
                self.start_task(matching_task_member, instant, rough_destination, world);
            }
            fate => {
                info(
                    LOG_T,
                    format!(
                        "Trip of member #{} from {:?} to {:?} failed ({:?})!\n",
                        matching_task_member.as_idx(),
                        rough_source,
                        rough_destination,
                        fate
                    ),
                    self.id(),
                    world,
                );

                if let Some((_, offer)) =
                    self.core().member_tasks[matching_task_member.as_idx()].goal
                {
                    offer.household.request_receive_undo_deal(
                        offer.idx,
                        self.id_as(),
                        matching_task_member,
                        world,
                    );
                }
                self.stop_task(matching_task_member, result.location_now, world);
            }
        }
    }

    fn start_task(
        &mut self,
        member: MemberIdx,
        start: Instant,
        location: RoughLocationID,
        world: &mut World,
    ) {
        debug(
            LOG_T,
            format!("Member #{} started task", member.0),
            self.id(),
            world,
        );
        TaskEndSchedulerID::local_first(world).schedule(
            start + self.core().member_tasks[member.as_idx()].duration,
            self.id_as(),
            member,
            world,
        );
        if let Some((_, offer)) = self.core().member_tasks[member.as_idx()].goal {
            offer
                .household
                .started_actively_using(offer.idx, self.id_as(), member, world);
        }
        self.core_mut().member_tasks[member.as_idx()].state = TaskState::StartedAt(start, location);
    }

    fn stop_task(
        &mut self,
        member: MemberIdx,
        location: Option<RoughLocationID>,
        world: &mut World,
    ) {
        if let TaskState::InTrip(trip) = self.core().member_tasks[member.as_idx()].state {
            debug(LOG_T, "Force stopping trip", self.id(), world);
            // reuse normal trip failed behaviour
            trip.finish(
                TripResult {
                    location_now: None,
                    fate: TripFate::ForceStopped,
                },
                world,
            )
        } else {
            let old_state = self.core().member_tasks[member.as_idx()].state;
            debug(
                LOG_T,
                format!(
                    "Task of member {} stopped (was in state {:?})\n",
                    member.as_idx(),
                    old_state,
                ),
                self.id(),
                world,
            );

            self.core_mut().member_tasks[member.as_idx()].state =
                TaskState::IdleAt(location.unwrap_or_else(|| self.site()));

            if let Some((_, offer)) = self.core().member_tasks[member.as_idx()].goal {
                offer
                    .household
                    .stopped_actively_using(offer.idx, self.id_as(), member, world);
            }

            TimeID::local_first(world).wake_up_in(Ticks(0), self.id_as(), world);
        }
    }

    fn on_tick(&mut self, current_instant: Instant, world: &mut World) {
        if (current_instant.ticks() + self.id().as_raw().instance_id as usize)
            % (UPDATE_EVERY_N_SECS * TICKS_PER_SIM_SECOND) as usize
            == 0
        {
            self.decay(Duration(UPDATE_EVERY_N_SECS * TICKS_PER_SIM_SECOND), world);
        }
    }

    fn get_offer(&self, idx: OfferIdx) -> &Offer {
        &self.core().provided_offers[idx.0 as usize]
    }

    fn get_offer_mut(&mut self, idx: OfferIdx) -> &mut Offer {
        &mut self.core_mut().provided_offers[idx.0 as usize]
    }

    fn evaluate(
        &mut self,
        offer_idx: OfferIdx,
        instant: Instant,
        location: RoughLocationID,
        requester: EvaluationRequesterID,
        world: &mut World,
    ) {
        let offer = self.get_offer(offer_idx);

        if offer
            .opening_hours
            .end_after_on_same_day(TimeOfDay::from(instant))
        {
            let search_result = EvaluatedSearchResult {
                resource: offer.deal.main_given(),
                evaluated_deals: vec![EvaluatedDeal {
                    offer: OfferID {
                        household: self.id_as(),
                        idx: offer_idx,
                    },
                    deal: offer.deal.clone(),
                    opening_hours: offer.opening_hours,
                }]
                .into(),
            };
            TripCostEstimatorID::spawn(
                requester,
                location,
                self.site(),
                search_result,
                instant,
                world,
            );
        } else {
            debug(
                LOG_T,
                format!("Not in opening hours for {}", offer.deal.main_given()),
                self.id(),
                world,
            );
            requester.on_result(
                EvaluatedSearchResult {
                    resource: offer.deal.main_given(),
                    evaluated_deals: CVec::new(),
                },
                world,
            );
        }
    }

    fn request_receive_deal(
        &mut self,
        offer_idx: OfferIdx,
        requester: HouseholdID,
        requester_member: MemberIdx,
        world: &mut World,
    ) {
        let offer = self.get_offer(offer_idx).clone(); // borrow checker too dumb
        self.provide_deal(&offer.deal, offer.offering_member, world);
        requester.receive_deal(offer.deal, requester_member, world);
    }

    fn request_receive_undo_deal(
        &mut self,
        offer_idx: OfferIdx,
        requester: HouseholdID,
        requester_member: MemberIdx,
        world: &mut World,
    ) {
        let offer = self.get_offer(offer_idx).clone(); // borrow checker too dumb
        self.receive_deal(&offer.deal, offer.offering_member, world);
        requester.provide_deal(offer.deal, requester_member, world);
    }

    fn started_using(
        &mut self,
        offer_idx: OfferIdx,
        user: HouseholdID,
        using_member: Option<MemberIdx>,
        world: &mut World,
    ) {
        let id_as_household = self.id_as();
        let offer = self.get_offer_mut(offer_idx);
        if !offer.users.contains(&(user, using_member)) {
            offer.users.push((user, using_member));
            if !offer.is_internal && offer.users.len() >= offer.max_users as usize {
                MarketID::global_first(world).withdraw(
                    offer.deal.main_given(),
                    OfferID {
                        household: id_as_household,
                        idx: offer_idx,
                    },
                    world,
                );
                // already too much!
                // TODO: this is a super hacky way to undo the overuse that happens when a lot
                // households spawn at the same time. Replace this with a proper contract where the
                // households waits for confirmation that it can indeed start using this offer
                if offer.users.len() > offer.max_users as usize {
                    user.stop_using(
                        OfferID {
                            household: id_as_household,
                            idx: offer_idx,
                        },
                        world,
                    )
                }
            }
        }
    }

    fn stopped_using(
        &mut self,
        offer_idx: OfferIdx,
        user: HouseholdID,
        using_member: Option<MemberIdx>,
        world: &mut World,
    ) -> Fate {
        {
            let id_as_household = self.id_as();
            let offer = self.get_offer_mut(offer_idx);
            let users_before = offer.users.len();

            offer.users.retain(|&(o_user, o_using_member)| {
                o_user != user || o_using_member != using_member
            });

            if offer.is_internal
                && users_before >= offer.max_users as usize
                && offer.users.len() < offer.max_users as usize
            {
                MarketID::global_first(world).register(
                    offer.deal.main_given(),
                    OfferID {
                        household: id_as_household,
                        idx: offer_idx,
                    },
                    world,
                );
            }
        }

        if self.core().being_destroyed {
            // maybe already all users are gone
            let no_users = |offer: &Offer| offer.users.is_empty();
            if self.core().provided_offers.iter().all(no_users) {
                Fate::Die
            } else {
                Fate::Live // for now
            }
        } else {
            Fate::Live
        }
    }

    fn started_actively_using(
        &mut self,
        offer_idx: OfferIdx,
        user: HouseholdID,
        using_member: MemberIdx,
        _: &mut World,
    ) {
        let offer = self.get_offer_mut(offer_idx);
        if !offer.active_users.contains(&(user, using_member)) {
            offer.active_users.push((user, using_member));
        }
    }

    fn stopped_actively_using(
        &mut self,
        offer_idx: OfferIdx,
        user: HouseholdID,
        using_member: MemberIdx,
        _: &mut World,
    ) {
        let offer = self.get_offer_mut(offer_idx);
        offer
            .active_users
            .retain(|&(o_user, o_using_member)| o_user != user || o_using_member != using_member);
    }

    // TODO: there is still a tiny potential race condition here:
    //       1) household finds offer in market -> household
    //       2) offer withdrawn from market
    //       3) withdrawal confirmed
    //       ... starting to notify existing users
    //       4) household starts using offer
    //       => dangling single user keeping the offer half-dead
    fn withdrawal_confirmed(&mut self, offer_idx: OfferIdx, world: &mut World) -> Fate {
        if self.core().being_destroyed {
            let offer = self.get_offer(offer_idx);

            for user in &offer.users {
                user.0.stop_using(
                    OfferID {
                        household: self.id_as(),
                        idx: offer_idx,
                    },
                    world,
                )
            }

            // maybe already all users are gone
            let no_users = |offer: &Offer| offer.users.is_empty();
            if self.core().provided_offers.iter().all(no_users) {
                Fate::Die
            } else {
                Fate::Live // for now
            }
        } else {
            Fate::Live
        }
    }

    fn get_ui_info(&mut self, requester: ui::HouseholdUIID, world: &mut World) {
        requester.on_household_ui_info(self.id_as(), self.core().clone(), world);
    }
}

#[derive(Compact, Clone)]
pub enum ResultAspect {
    AddDeals(CVec<EvaluatedDeal>),
    SetTarget(u32),
}

#[derive(Compact, Clone, Serialize)]
pub struct DecisionResourceEntry {
    results_counter: AsyncCounter,
    best_deal: COption<EvaluatedDeal>,
    best_deal_usefulness: f32,
}

#[derive(Compact, Clone, Serialize)]
pub enum DecisionState {
    None,
    Choosing(
        MemberIdx,
        Instant,
        CVec<(Resource, f32)>,
        CDict<Resource, DecisionResourceEntry>,
    ),
    WaitingForTrip(MemberIdx),
}

#[derive(Compact, Clone, Serialize)]
pub struct HouseholdCore {
    pub resources: Inventory,
    pub member_resources: CVec<Inventory>,
    pub member_tasks: CVec<Task>,
    pub decision_state: DecisionState,
    pub used_offers: ResourceMap<OfferID>,
    pub member_used_offers: CVec<ResourceMap<OfferID>>,
    pub provided_offers: CVec<Offer>,
    pub being_destroyed: bool,
}

impl HouseholdCore {
    pub fn new(
        owner: HouseholdID,
        world: &mut World,
        n_members: usize,
        initial_location: RoughLocationID,
        provided_offers: CVec<Offer>,
    ) -> Self {
        assert!(n_members > 0);

        for (idx, offer) in provided_offers.iter().enumerate() {
            MarketID::local_first(world).register(
                offer.deal.main_given(),
                OfferID {
                    household: owner,
                    idx: OfferIdx(idx as u16),
                },
                world,
            )
        }

        HouseholdCore {
            resources: Inventory::new(),
            member_resources: vec![Inventory::new(); n_members].into(),
            member_tasks: vec![Task::idle_at(initial_location); n_members].into(),
            decision_state: DecisionState::None,
            used_offers: ResourceMap::new(),
            member_used_offers: vec![ResourceMap::new(); n_members].into(),
            provided_offers,
            being_destroyed: false,
        }
    }
}

pub fn setup(system: &mut ActorSystem) {
    auto_setup(system);
    tasks::setup(system);
    family::setup(system);
    grocery_shop::setup(system);
    grain_farm::setup(system);
    cow_farm::setup(system);
    vegetable_farm::setup(system);
    mill::setup(system);
    bakery::setup(system);
    neighboring_town_trade::setup(system);
    ui::auto_setup(system);
}

pub fn spawn(world: &mut World) {
    tasks::spawn(world);
}

mod kay_auto;
pub use self::kay_auto::*;
