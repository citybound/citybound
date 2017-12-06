use kay::{ActorSystem, World, Actor, TypedID, Fate};
use compact::{CVec, CDict, COption, CString};
use core::simulation::{Duration, TimeOfDay, Instant, Ticks, Simulation, TICKS_PER_SIM_SECOND,
                       Sleeper, Simulatable};
use core::async_counter::AsyncCounter;
use core::random::{seed, Rng};
use ordered_float::OrderedFloat;

pub mod tasks;
pub mod offers;

pub mod family;
pub mod grocery_shop;
pub mod grain_farm;
pub mod cow_farm;
pub mod vegetable_farm;
pub mod mill;
pub mod bakery;
pub mod neighboring_town_trade;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct MemberIdx(usize);

use imgui::Ui;
use kay::External;

use super::market::{Market, Deal, EvaluatedDeal, EvaluationRequester, EvaluationRequesterID,
                    TripCostEstimatorID, EvaluatedSearchResult};
use super::buildings::rendering::BuildingInspectorID;
use super::resources::{Resource, ResourceAmount, ResourceMap, Entry, Inventory};
use transport::pathfinding::{RoughLocationID, RoughLocation};
use transport::pathfinding::trip::{TripListener, TripID, TripResult, TripFate};
use self::tasks::{Task, TaskState, TaskEndScheduler};
pub use self::offers::{Offer, OfferIdx, OfferID};

const N_TOP_PROBLEMS: usize = 5;
const DECISION_PAUSE: Ticks = Ticks(200);
const UPDATE_EVERY_N_SECS: usize = 4;

// TODO: make kay_codegen figure this out on it's own
impl Into<RoughLocationID> for HouseholdID {
    fn into(self) -> RoughLocationID {
        unsafe { RoughLocationID::from_raw(self.as_raw()) }
    }
}

pub trait Household
    : Actor + EvaluationRequester + Sleeper + Simulatable + TripListener + RoughLocation
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
            &mut core.member_resources[member.0],
            Self::is_shared,
        );
    }

    fn provide_deal(&mut self, deal: &Deal, member: MemberIdx, _: &mut World) {
        let core = self.core_mut();
        let provide_awakeness = deal.delta.len() == 1 &&
            deal.delta.get(Resource::Awakeness).is_some();
        if !provide_awakeness {
            deal.delta.take_from_shared_private(
                &mut core.resources,
                &mut core.member_resources[member.0],
                Self::is_shared,
            );
        }
    }

    fn task_succeeded(&mut self, member: MemberIdx, world: &mut World) {
        {
            self.core_mut().log.log("Task succeeded\n");
            if let TaskState::StartedAt(_, location) = self.core().member_tasks[member.0].state {
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
        self.core_mut().log.log(
            format!("Reset member {}\n", member.0)
                .as_str(),
        );
        TaskEndScheduler::local_first(world).deschedule(self.id_as(), member, world);

        self.stop_task(member, None, world);
    }

    fn stop_using(&mut self, offer: OfferID, world: &mut World) {
        if let Some(Entry(associated_resource, _)) =
            self.core()
                .used_offers
                .iter()
                .find(|&&Entry(_, resource_offer)| resource_offer == offer)
                .cloned()
        {
            self.core_mut().used_offers.remove(associated_resource);
            offer.household.stopped_using(
                offer.idx,
                self.id_as(),
                None,
                world,
            );
        }

        let id_as_household = self.id_as();

        for (i, member_used_offers) in self.core_mut().member_used_offers.iter_mut().enumerate() {
            if let Some(Entry(associated_resource, _)) =
                member_used_offers
                    .iter()
                    .find(|&&Entry(_, resource_offer)| resource_offer == offer)
                    .cloned()
            {
                member_used_offers.remove(associated_resource);
                offer.household.stopped_using(
                    offer.idx,
                    id_as_household,
                    Some(MemberIdx(i)),
                    world,
                );
            }
        }

        let members_to_reset = self.core()
            .member_tasks
            .iter()
            .enumerate()
            .filter_map(|(i, task)| if let Task {
                goal: Some((_, task_offer)), ..
            } = *task
            {
                if task_offer == offer {
                    Some(MemberIdx(i))
                } else {
                    None
                }
            } else {
                None
            })
            .collect::<Vec<_>>();

        for member_to_reset in members_to_reset {
            self.reset_member_task(member_to_reset, world);
        }
    }

    fn destroy(&mut self, world: &mut World) {
        self.core_mut().being_destroyed = true;

        for &Entry(_, offer) in self.core().used_offers.iter() {
            offer.household.stopped_using(
                offer.idx,
                self.id_as(),
                None,
                world,
            );
        }

        for (i, member_used_offers) in self.core().member_used_offers.iter().enumerate() {
            for &Entry(_, offer) in member_used_offers.iter() {
                offer.household.stopped_using(
                    offer.idx,
                    self.id_as(),
                    Some(MemberIdx(i)),
                    world,
                );
            }
        }

        for (idx, offer) in self.core().provided_offers.iter().enumerate() {
            Market::local_first(world).withdraw(
                offer.deal.main_given(),
                OfferID {
                    household: self.id_as(),
                    idx: OfferIdx(idx),
                },
                world,
            )
        }

        self.on_destroy(world);
    }
    fn on_destroy(&mut self, world: &mut World);

    fn update_core(&mut self, current_instant: Instant, world: &mut World) {
        if let DecisionState::None = self.core().decision_state {
            let idle_members_idx_loc = self.core()
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
                    MemberIdx(idle_member_idx),
                    current_instant,
                    location,
                    world,
                );
            }
        };
    }

    fn top_problems(&self, member: MemberIdx, time: TimeOfDay) -> Vec<(Resource, f32)> {
        let mut resource_graveness = self.core()
            .resources
            .iter()
            .chain(self.core().member_resources[member.0].iter())
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
        self.core_mut().log.log("Top N Problems\n");

        let time = TimeOfDay::from(instant);
        let top_problems = self.top_problems(member, time);

        if top_problems.is_empty() {
            Simulation::local_first(world).wake_up_in(DECISION_PAUSE, self.id_as(), world);
        } else {
            let mut decision_entries = CDict::<Resource, DecisionResourceEntry>::new();
            let id_as_eval_requester = self.id_as();
            let core = self.core_mut();

            for &(resource, graveness) in &top_problems {
                core.log.log(
                    format!("Member #{}: {} = {}", member.0, resource, graveness).as_str(),
                );
                let maybe_offer = if Self::supplier_shared(resource) {
                    core.used_offers.get(resource)
                } else {
                    core.member_used_offers[member.0].get(resource)
                };

                let initial_counter = if let Some(&offer) = maybe_offer {
                    core.log.log(
                        format!(" -> Using favorite offer {:?} for {}\n", offer, resource)
                            .as_str(),
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
                    core.log.log(
                        format!(" -> Doing market query for {}\n", resource).as_str(),
                    );
                    Market::global_first(world).search(
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
            let core = self.core_mut();

            if let DecisionState::Choosing(_, instant, ref top_problems, ref mut entries) =
                core.decision_state
            {
                {
                    let entry = entries.get_mut(resource).expect(
                        "Should have an entry for queried resource",
                    );

                    match *update {
                        ResultAspect::AddDeals(ref evaluated_deals) => {
                            for evaluated_deal in evaluated_deals {
                                core.log.log(
                                    format!("Got eval'd deal for {}, {:?} -> {:?}\n",
                                        evaluated_deal.deal.main_given(),
                                        evaluated_deal.opening_hours.start.hours_minutes(),
                                        evaluated_deal.opening_hours.end.hours_minutes(),).as_str(),
                                );
                                if evaluated_deal.opening_hours.contains(instant) {
                                    let new_deal_usefulness = Self::deal_usefulness(
                                        &mut core.log,
                                        top_problems,
                                        evaluated_deal,
                                    );
                                    if new_deal_usefulness > entry.best_deal_usefulness {
                                        entry.best_deal = COption(Some(evaluated_deal.clone()));
                                        entry.best_deal_usefulness = new_deal_usefulness;
                                    } else {
                                        core.log.log(
                                            format!(
                                                "Deal rejected, not more useful: {} vs {}\n",
                                                new_deal_usefulness,
                                                entry.best_deal_usefulness
                                            ).as_str(),
                                        );
                                    }
                                } else {
                                    core.log.log("Deal rejected: not open\n");
                                }
                            }

                            entry.results_counter.increment();
                        }
                        ResultAspect::SetTarget(n) => {
                            entry.results_counter.set_target(n);
                        }
                    }
                }

                entries.values().all(
                    |entry| entry.results_counter.is_done(),
                )
            } else {
                core.log.log(
                    "Received unexpected deal / should be choosing\n",
                );
                false
            }
        };

        if done {
            self.choose_deal(world);
        }
    }

    fn deal_usefulness(
        log: &mut HouseholdLog,
        top_problems: &[(Resource, f32)],
        evaluated: &EvaluatedDeal,
    ) -> f32 {
        let resource_graveness_improvement: f32 = top_problems
            .iter()
            .map(|&(resource, graveness)| {
                let delta = evaluated.deal.delta.get(resource).cloned().unwrap_or(0.0);
                let improvement_strength = delta * graveness;
                log.log(
                    format!(
                        "{} improves by {} (graveness {}, delta: {:?})\n",
                        resource,
                        improvement_strength,
                        graveness,
                        evaluated.deal.delta.get(resource)
                    ).as_str(),
                );
                improvement_strength
            })
            .sum();

        resource_graveness_improvement / evaluated.deal.duration.as_seconds()
    }

    fn choose_deal(&mut self, world: &mut World) {
        let id_as_household = self.id_as();
        let id_as_sleeper = self.id_as();
        self.core_mut().log.log("Choosing deal!\n");

        let maybe_best_info = {
            let core = self.core_mut();

            if let DecisionState::Choosing(member, instant, _, ref entries) = core.decision_state {
                let maybe_best = most_useful_evaluated_deal(entries);

                if let Some(best) = maybe_best {
                    let task = &mut core.member_tasks[member.0];

                    *task = if let TaskState::IdleAt(location) = task.state {
                        Task {
                            goal: Some((best.deal.main_given(), best.offer)),
                            duration: best.deal.duration,
                            state: TaskState::GettingReadyAt(location),
                        }
                    } else {
                        panic!("Member who gets new task should be idle");
                    };

                    core.log.log(
                        format!("Found best offer for {}\n", best.deal.main_given()).as_str(),
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
                let (used_offers, maybe_member) =
                    if Self::supplier_shared(best.deal.main_given()) {
                        (&mut self.core_mut().used_offers, None)
                    } else {
                        (
                            &mut self.core_mut().member_used_offers[member.0],
                            Some(member),
                        )
                    };
                if let Some(previous_offer) =
                    used_offers.insert(best.deal.main_given(), best.offer)
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
            self.core_mut().log.log(
                "Didn't find any suitable offers at all\n",
            );
            self.core_mut().decision_state = DecisionState::None;
            Simulation::local_first(world).wake_up_in(DECISION_PAUSE, id_as_sleeper, world);
        }

        fn most_useful_evaluated_deal(
            entries: &CDict<Resource, DecisionResourceEntry>,
        ) -> Option<EvaluatedDeal> {
            entries
                .values()
                .max_by_key(|decision_entry| {
                    OrderedFloat(decision_entry.best_deal_usefulness)
                })
                .and_then(|best_entry| best_entry.best_deal.as_ref().cloned())
        }
    }

    fn start_trip(&mut self, member: MemberIdx, instant: Instant, world: &mut World) {
        if let Task {
            goal: Some((_, offer)),
            state: TaskState::GettingReadyAt(source),
            ..
        } = self.core().member_tasks[member.0]
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
                self.core_mut().member_tasks[member.0].state = TaskState::InTrip(trip);
                Simulation::local_first(world).wake_up_in(DECISION_PAUSE, self.id_as(), world);
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
        let (matching_task_member, matching_resource, matching_offer) =
            self.core()
                .member_tasks
                .iter()
                .enumerate()
                .filter_map(|(idx, task)| if let TaskState::InTrip(task_trip_id) =
                    task.state
                {
                    if task_trip_id == trip {
                        if let Some((goal, offer)) = task.goal {
                            Some((MemberIdx(idx), goal, offer))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                })
                .next()
                .expect("Should have a matching task");
        {
            let id_as_household = self.id_as();

            let (used_offers, maybe_member) = if Self::supplier_shared(matching_resource) {
                (&mut self.core_mut().used_offers, None)
            } else {
                (
                    &mut self.core_mut().member_used_offers[matching_task_member.0],
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
                self.core_mut().log.log(
                    format!(
                        "Trip of member #{} from {:?} to {:?} failed ({:?})!\n",
                        matching_task_member.0,
                        rough_source,
                        rough_destination,
                        fate
                    ).as_str(),
                );

                if let Some((_, offer)) = self.core().member_tasks[matching_task_member.0].goal {
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
        self.core_mut().log.log("Started task\n");
        TaskEndScheduler::local_first(world).schedule(
            start + self.core().member_tasks[member.0].duration,
            self.id_as(),
            member,
            world,
        );
        if let Some((_, offer)) = self.core().member_tasks[member.0].goal {
            offer.household.started_actively_using(
                offer.idx,
                self.id_as(),
                member,
                world,
            );
        }
        self.core_mut().member_tasks[member.0].state = TaskState::StartedAt(start, location);
    }

    fn stop_task(
        &mut self,
        member: MemberIdx,
        location: Option<RoughLocationID>,
        world: &mut World,
    ) {
        if let TaskState::InTrip(trip) = self.core().member_tasks[member.0].state {
            self.core_mut().log.log("Force stopping trip\n");
            // reuse normal trip failed behaviour
            trip.finish(
                TripResult {
                    location_now: None,
                    fate: TripFate::ForceStopped,
                },
                world,
            )
        } else {
            let old_state = self.core().member_tasks[member.0].state;
            self.core_mut().log.log(
                format!(
                "Task of member {} stopped (was in state {:?})\n",
                member.0,
                old_state,
            ).as_str(),
            );

            self.core_mut().member_tasks[member.0].state =
                TaskState::IdleAt(location.unwrap_or_else(|| self.site()));

            if let Some((_, offer)) = self.core().member_tasks[member.0].goal {
                offer.household.stopped_actively_using(
                    offer.idx,
                    self.id_as(),
                    member,
                    world,
                );
            }

            Simulation::local_first(world).wake_up_in(Ticks(0), self.id_as(), world);
        }
    }

    fn on_tick(&mut self, current_instant: Instant, world: &mut World) {
        if (current_instant.ticks() + self.id().as_raw().instance_id as usize) %
            (UPDATE_EVERY_N_SECS * TICKS_PER_SIM_SECOND) == 0
        {
            self.decay(Duration(UPDATE_EVERY_N_SECS * TICKS_PER_SIM_SECOND), world);
        }
    }

    fn get_offer(&self, idx: OfferIdx) -> &Offer {
        &self.core().provided_offers[idx.0]
    }

    fn get_offer_mut(&mut self, idx: OfferIdx) -> &mut Offer {
        &mut self.core_mut().provided_offers[idx.0]
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

        if offer.opening_hours.end_after_on_same_day(
            TimeOfDay::from(instant),
        )
        {
            let search_result = EvaluatedSearchResult {
                resource: offer.deal.main_given(),
                evaluated_deals: vec![
                    EvaluatedDeal {
                        offer: OfferID { household: self.id_as(), idx: offer_idx },
                        deal: offer.deal.clone(),
                        opening_hours: offer.opening_hours,
                    },
                ].into(),
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
            // println!(
            //     "Not in opening hours for {}",
            //     r_info(offer.deal.main_given()).0
            // );
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
            if !offer.is_internal && offer.users.len() >= offer.max_users {
                Market::global_first(world).withdraw(
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
                if offer.users.len() > offer.max_users {
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

            if offer.is_internal && users_before >= offer.max_users &&
                offer.users.len() < offer.max_users
            {
                Market::global_first(world).register(
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
        offer.active_users.retain(|&(o_user, o_using_member)| {
            o_user != user || o_using_member != using_member
        });
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
                    OfferID { household: self.id_as(), idx: offer_idx },
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

    #[allow(useless_format)]
    fn inspect(
        &mut self,
        imgui_ui: &External<Ui<'static>>,
        return_to: BuildingInspectorID,
        world: &mut World,
    ) {
        let ui = imgui_ui.steal();

        ui.window(im_str!("Building")).build(|| {
            ui.tree_node(im_str!("{}", self.household_name())).build(
                || {
                    // ui.text(im_str!(
                    //     "({})",
                    //     match self.decision_state {
                    //         DecisionState::None => "",
                    //         DecisionState::Choosing(_, _, _, _) => ": Waiting for choice",
                    //         DecisionState::WaitingForTrip(_) => ": Waiting for trip",
                    //     }
                    // ));
                    for resource in Self::interesting_resources() {
                        if Self::is_shared(*resource) {
                            ui.text(im_str!("{}", resource));
                            ui.same_line(130.0);
                            let amount =
                                self.core().resources.get(*resource).cloned().unwrap_or(0.0);
                            ui.text(im_str!("{:.2}", amount));
                        }
                    }
                    for (i, (member_resources, member_task)) in
                        self.core()
                            .member_resources
                            .iter()
                            .zip(&self.core().member_tasks)
                            .enumerate()
                    {
                        ui.spacing();
                        ui.text(im_str!(
                            "{}:",
                            self.member_name(MemberIdx(i)),
                        ));
                        ui.text(im_str!(
                            "({} {})",
                            match member_task.state {
                                TaskState::IdleAt(_) => "Idle after getting",
                                TaskState::GettingReadyAt(_) => "Preparing to get",
                                TaskState::InTrip(_) => "In trip to get",
                                TaskState::StartedAt(_, _) => "Getting",
                            },
                            member_task
                                .goal
                                .map(|goal| format!("{}", goal.0))
                                .unwrap_or_else(|| "nothing".to_owned())
                        ));
                        for resource in Self::interesting_resources() {
                            if !Self::is_shared(*resource) {
                                ui.text(im_str!("{}", resource));
                                ui.same_line(130.0);
                                let amount =
                                    member_resources.get(*resource).cloned().unwrap_or(0.0);
                                ui.text(im_str!("{:.2}", amount));
                            }
                        }
                    }
                    ui.tree_node(im_str!("Offers")).build(|| for offer in
                        &self.core().provided_offers
                    {
                        ui.text(im_str!("Offer for {}", offer.deal.main_given()));
                        ui.text(im_str!(
                            "{} of max. {} users ({} right now)",
                            offer.users.len(),
                            offer.max_users,
                            offer.active_users.len()
                        ))
                    });
                    ui.tree_node(im_str!("Log")).build(
                        || for line in self.core_mut()
                            .log
                            .0
                            .lines()
                        {
                            ui.text(im_str!("{}", line));
                        },
                    );
                },
            )
        });

        return_to.ui_drawn(ui, world);
    }
}

#[derive(Compact, Clone)]
pub enum ResultAspect {
    AddDeals(CVec<EvaluatedDeal>),
    SetTarget(usize),
}

#[derive(Compact, Clone)]
struct DecisionResourceEntry {
    results_counter: AsyncCounter,
    best_deal: COption<EvaluatedDeal>,
    best_deal_usefulness: f32,
}

#[derive(Compact, Clone)]
enum DecisionState {
    None,
    Choosing(MemberIdx, Instant, CVec<(Resource, f32)>, CDict<Resource, DecisionResourceEntry>),
    WaitingForTrip(MemberIdx),
}

#[derive(Compact, Clone)]
pub struct HouseholdCore {
    resources: Inventory,
    member_resources: CVec<Inventory>,
    member_tasks: CVec<Task>,
    decision_state: DecisionState,
    used_offers: ResourceMap<OfferID>,
    member_used_offers: CVec<ResourceMap<OfferID>>,
    provided_offers: CVec<Offer>,
    log: HouseholdLog,
    being_destroyed: bool,
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
            Market::local_first(world).register(
                offer.deal.main_given(),
                OfferID { household: owner, idx: OfferIdx(idx) },
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
            log: HouseholdLog(CString::new()),
            being_destroyed: false,
        }
    }
}

const DO_HOUSEHOLD_LOGGING: bool = true;

#[derive(Compact, Clone, Default)]
pub struct HouseholdLog(CString);

impl HouseholdLog {
    pub fn log(&mut self, string: &str) {
        if DO_HOUSEHOLD_LOGGING {
            self.0.push_str(string);
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
}

mod kay_auto;
pub use self::kay_auto::*;
