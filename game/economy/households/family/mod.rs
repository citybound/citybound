use kay::{ActorSystem, World, External};
use compact::{CVec, CDict, COption, CString};
use imgui::Ui;
use ordered_float::OrderedFloat;
use core::random::{seed, Rng};

use core::simulation::{TimeOfDay, TimeOfDayRange, Instant, Duration, Ticks, SimulationID,
                       Simulatable, SimulatableID, MSG_Simulatable_tick};
use economy::resources::{Resource, ResourceAmount, ResourceMap, Entry};
use economy::market::{Deal, MarketID, OfferID, EvaluatedDeal, EvaluationRequester,
                      EvaluationRequesterID, MSG_EvaluationRequester_expect_n_results,
                      MSG_EvaluationRequester_on_result, EvaluatedSearchResult};
use economy::buildings::BuildingID;
use economy::buildings::rendering::BuildingInspectorID;
use transport::pathfinding::trip::{TripResult, TripFate, TripListenerID,
                                   MSG_TripListener_trip_created, MSG_TripListener_trip_result};
use transport::pathfinding::RoughLocationID;

pub mod names;
use self::names::{family_name, member_name};

use core::async_counter::AsyncCounter;

use super::{Household, HouseholdID, MemberIdx, MSG_Household_decay, MSG_Household_inspect,
            MSG_Household_provide_deal, MSG_Household_receive_deal, MSG_Household_task_succeeded,
            MSG_Household_task_failed, MSG_Household_destroy, MSG_Household_stop_using,
            MSG_Household_reset_member_task};
use super::tasks::{Task, TaskEndSchedulerID};

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

const DO_FAMILY_LOGGING: bool = true;

#[derive(Compact, Clone, Default)]
pub struct FamilyLog(CString);

impl FamilyLog {
    pub fn log(&mut self, string: &str) {
        if DO_FAMILY_LOGGING {
            self.0.push_str(string);
        }
    }
}

#[derive(Compact, Clone)]
pub struct Family {
    id: FamilyID,
    home: BuildingID,
    sleep_offer: OfferID,
    resources: ResourceMap<ResourceAmount>,
    member_resources: CVec<ResourceMap<ResourceAmount>>,
    member_tasks: CVec<Task>,
    decision_state: DecisionState,
    used_offers: ResourceMap<OfferID>,
    member_used_offers: CVec<ResourceMap<OfferID>>,
    log: FamilyLog,
}

const N_TOP_PROBLEMS: usize = 5;
const DECISION_PAUSE: Ticks = Ticks(200);
const UPDATE_EVERY_N_SECS: usize = 4;

impl Family {
    pub fn move_into(
        id: FamilyID,
        n_members: usize,
        home: BuildingID,
        simulation: SimulationID,
        world: &mut World,
    ) -> Family {
        simulation.wake_up_in(Ticks(0), id.into(), world);

        let sleep_offer = OfferID::internal(
            id.into(),
            MemberIdx(0),
            home.into(),
            TimeOfDayRange::new(16, 0, 11, 0),
            Deal::new(Some((Resource::Awakeness, 3.0)), Duration::from_hours(1)),
            world,
        );

        let mut used_offers = ResourceMap::new();
        used_offers.insert(Resource::Awakeness, sleep_offer);

        Family {
            id,
            home,
            sleep_offer,
            resources: ResourceMap::new(),
            member_resources: vec![ResourceMap::new(); n_members].into(),
            member_tasks: vec![Task::idle_at(home.into()); n_members].into(),
            decision_state: DecisionState::None,
            used_offers,
            member_used_offers: vec![ResourceMap::new(); n_members].into(),
            log: FamilyLog::default(),
        }
    }
}

use core::simulation::{Sleeper, SleeperID, MSG_Sleeper_wake};

impl Sleeper for Family {
    fn wake(&mut self, current_instant: Instant, world: &mut World) {
        if let DecisionState::None = self.decision_state {
            let idle_members_idx_loc = self.member_tasks
                .iter()
                .enumerate()
                .filter_map(|(idx, m)| match m.state {
                    TaskState::IdleAt(loc) => Some((idx, loc)),
                    _ => None,
                })
                .collect::<Vec<_>>();
            let mut rng = seed((current_instant.ticks(), self.id));
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
}

impl Family {
    pub fn top_problems(&self, member: MemberIdx, time: TimeOfDay) -> Vec<(Resource, f32)> {
        let mut resource_graveness = self.resources
            .iter()
            .chain(self.member_resources[member.0].iter())
            .map(|&Entry(resource, amount)| {
                (resource, Self::graveness(resource, amount, time))
            })
            .collect::<Vec<_>>();
        resource_graveness.sort_by_key(|&(_r, i)| OrderedFloat(i));

        resource_graveness.truncate(N_TOP_PROBLEMS);
        resource_graveness
    }

    pub fn find_new_task_for(
        &mut self,
        member: MemberIdx,
        instant: Instant,
        location: RoughLocationID,
        world: &mut World,
    ) {
        self.log.log(
            format!("Top N Problems for Family {:?}\n", self.id._raw_id).as_str(),
        );

        let time = TimeOfDay::from(instant);
        let top_problems = self.top_problems(member, time);

        if top_problems.is_empty() {
            SimulationID::local_first(world).wake_up_in(DECISION_PAUSE, self.id.into(), world);
        } else {
            let mut decision_entries = CDict::<Resource, DecisionResourceEntry>::new();

            for &(resource, graveness) in &top_problems {
                self.log.log(
                    format!("Member #{}: {} = {}", member.0, resource, graveness).as_str(),
                );
                let maybe_offer = if Self::supplier_shared(resource) {
                    self.used_offers.get(resource)
                } else {
                    self.member_used_offers[member.0].get(resource)
                };

                let initial_counter = if let Some(&offer) = maybe_offer {
                    self.log.log(
                        format!(
                            " -> Using favorite offer {:?} for {}\n",
                            offer._raw_id,
                            resource
                        ).as_str(),
                    );
                    offer.evaluate(instant, location, self.id.into(), world);

                    AsyncCounter::with_target(1)
                } else {
                    self.log.log(
                        format!(" -> Doing market query for {}\n", resource).as_str(),
                    );
                    MarketID::global_first(world).search(
                        instant,
                        location,
                        resource,
                        self.id.into(),
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

            self.decision_state =
                DecisionState::Choosing(member, instant, top_problems.into(), decision_entries);
        }
    }
}

#[derive(Compact, Clone)]
enum ResultAspect {
    AddDeals(CVec<EvaluatedDeal>),
    SetTarget(usize),
}

impl Family {
    fn update_results(&mut self, resource: Resource, update: ResultAspect, world: &mut World) {
        let done =
            if let DecisionState::Choosing(_, instant, ref top_problems, ref mut entries) =
                self.decision_state
            {
                {
                    let entry = entries.get_mut(resource).expect(
                        "Should have an entry for queried resource",
                    );

                    match update {
                        ResultAspect::AddDeals(ref evaluated_deals) => {
                            for evaluated_deal in evaluated_deals {
                                self.log.log(
                                    format!("Got eval'd deal for {}, {:?} -> {:?}\n",
                                        evaluated_deal.deal.main_given(),
                                        evaluated_deal.opening_hours.start.hours_minutes(),
                                        evaluated_deal.opening_hours.end.hours_minutes(),).as_str(),
                                );
                                if evaluated_deal.opening_hours.contains(instant) {
                                    let new_deal_usefulness = Self::deal_usefulness(
                                        &mut self.log,
                                        top_problems,
                                        evaluated_deal,
                                    );
                                    if new_deal_usefulness > entry.best_deal_usefulness {
                                        entry.best_deal = COption(Some(evaluated_deal.clone()));
                                        entry.best_deal_usefulness = new_deal_usefulness;
                                    } else {
                                        self.log.log(
                                            format!(
                                                "Deal rejected, not more useful: {} vs {}\n",
                                                new_deal_usefulness,
                                                entry.best_deal_usefulness
                                            ).as_str(),
                                        );
                                    }
                                } else {
                                    self.log.log("Deal rejected: not open\n");
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
                self.log.log(
                    "Received unexpected deal / should be choosing\n",
                );
                false
            };

        if done {
            self.choose_deal(world);
        }
    }

    fn deal_usefulness(
        log: &mut FamilyLog,
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


        // let improvement: f32 = evaluated
        //     .deal
        //     .delta
        //     .iter()
        //     .map(|&Entry(resource, amount)| {
        //         let resource_improvement = resource_graveness_helper(resource, -amount, time);
        //         log.push_str(
        //             format!(
        //                 "{} improves by {}\n",
        //                 resource,
        //                 resource_improvement
        //             ).as_str(),
        //         );
        //         resource_improvement
        //     })
        //     .sum();

        resource_graveness_improvement / evaluated.deal.duration.as_seconds()
    }
}


impl EvaluationRequester for Family {
    fn expect_n_results(&mut self, resource: Resource, n: usize, world: &mut World) {
        self.update_results(resource, ResultAspect::SetTarget(n), world);
    }

    fn on_result(&mut self, result: &EvaluatedSearchResult, world: &mut World) {
        let &EvaluatedSearchResult { resource, ref evaluated_deals, .. } = result;
        self.update_results(
            resource,
            ResultAspect::AddDeals(evaluated_deals.clone()),
            world,
        );
    }
}

impl Family {
    pub fn choose_deal(&mut self, world: &mut World) {
        self.log.log("Choosing deal!\n");
        let maybe_best_info =
            if let DecisionState::Choosing(member, instant, _, ref entries) = self.decision_state {
                let maybe_best = most_useful_evaluated_deal(entries);

                if let Some(best) = maybe_best {
                    let task = &mut self.member_tasks[member.0];

                    *task = if let TaskState::IdleAt(location) = task.state {
                        Task {
                            goal: Some((best.deal.main_given(), best.offer)),
                            duration: best.deal.duration,
                            state: TaskState::GettingReadyAt(location),
                        }
                    } else {
                        panic!("Member who gets new task should be idle");
                    };

                    self.log.log(
                        format!("Found best offer for {}\n", best.deal.main_given()).as_str(),
                    );

                    Some((member, instant, best.offer))
                } else {
                    None
                }
            } else {
                panic!("Tried to choose deal while not deciding");
            };
        if let Some((member, instant, best_offer)) = maybe_best_info {
            self.decision_state = DecisionState::WaitingForTrip(member);
            best_offer.request_receive_deal(self.id.into(), member, world);
            self.start_trip(member, instant, world);
        } else {
            self.log.log(
                format!(
                    "{:?} didn't find any suitable offers at all\n",
                    self.id._raw_id
                ).as_str(),
            );
            self.decision_state = DecisionState::None;
            SimulationID::local_first(world).wake_up_in(DECISION_PAUSE, self.id.into(), world);
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

    pub fn start_trip(&mut self, member: MemberIdx, instant: Instant, world: &mut World) {
        if let Task {
            goal: Some((_, offer)),
            state: TaskState::GettingReadyAt(source),
            ..
        } = self.member_tasks[member.0]
        {
            TripID::spawn(source, offer.into(), Some(self.id.into()), instant, world);
        } else {
            panic!("Member should be getting ready before starting trip");
        }
    }
}

use transport::pathfinding::trip::{TripListener, TripID};
use super::tasks::TaskState;

impl TripListener for Family {
    fn trip_created(&mut self, trip: TripID, world: &mut World) {
        self.decision_state = if let DecisionState::WaitingForTrip(member) = self.decision_state {
            self.member_tasks[member.0].state = TaskState::InTrip(trip);
            SimulationID::local_first(world).wake_up_in(DECISION_PAUSE, self.id.into(), world);
            DecisionState::None
        } else {
            panic!("Should be in waiting for trip state")
        };
    }

    fn trip_result(
        &mut self,
        trip: TripID,
        result: TripResult,
        rough_source: RoughLocationID,
        rough_destination: RoughLocationID,
        world: &mut World,
    ) {
        let (matching_task_member, matching_resource, matching_offer) =
            self.member_tasks
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
            let (used_offers, maybe_member) = if Self::supplier_shared(matching_resource) {
                (&mut self.used_offers, None)
            } else {
                (
                    &mut self.member_used_offers[matching_task_member.0],
                    Some(matching_task_member),
                )
            };

            match result.fate {
                TripFate::Success => {
                    if let Some(previous_offer) =
                        used_offers.insert(matching_resource, matching_offer)
                    {
                        if previous_offer != matching_offer {
                            previous_offer.stopped_using(self.id.into(), maybe_member, world);
                        }
                    }
                    matching_offer.started_using(self.id.into(), maybe_member, world);
                }
                _ => {
                    used_offers.remove(matching_resource);
                    matching_offer.stopped_using(self.id.into(), maybe_member, world);
                }
            }
        }

        match result.fate {
            TripFate::Success => {
                self.start_task(
                    matching_task_member,
                    result.instant,
                    rough_destination,
                    world,
                );
            }
            _ => {
                self.log.log(
                    format!(
                        "Trip of member #{} from {:?} to {:?} failed!\n",
                        matching_task_member.0,
                        rough_source,
                        rough_destination
                    ).as_str(),
                );

                if let Some((_, offer)) = self.member_tasks[matching_task_member.0].goal {
                    offer.request_receive_undo_deal(self.id.into(), matching_task_member, world);
                }
                self.stop_task(matching_task_member, result.location_now, world);

            }
        }
    }
}

impl Family {
    pub fn start_task(
        &mut self,
        member: MemberIdx,
        start: Instant,
        location: RoughLocationID,
        world: &mut World,
    ) {
        self.log.log("Started task\n");
        TaskEndSchedulerID::local_first(world).schedule(
            start + self.member_tasks[member.0].duration,
            self.id.into(),
            member,
            world,
        );
        if let Some((_, offer)) = self.member_tasks[member.0].goal {
            offer.started_actively_using(self.id.into(), member, world);
        }
        self.member_tasks[member.0].state = TaskState::StartedAt(start, location);
    }

    pub fn stop_task(
        &mut self,
        member: MemberIdx,
        location: Option<RoughLocationID>,
        world: &mut World,
    ) {
        self.member_tasks[member.0].state =
            TaskState::IdleAt(location.unwrap_or_else(|| self.home.into()));
        self.log.log("Task stopped\n");
        if let Some((_, offer)) = self.member_tasks[member.0].goal {
            offer.stopped_actively_using(self.id.into(), member, world);
        }
        SimulationID::local_first(world).wake_up_in(Ticks(0), self.id.into(), world);
    }
}

impl Household for Family {
    fn is_shared(resource: Resource) -> bool {
        match resource {
            Resource::Awakeness | Resource::Satiety => false,
            Resource::Money | Resource::Groceries => true,
            _ => unimplemented!(),
        }
    }

    fn supplier_shared(resource: Resource) -> bool {
        match resource {
            Resource::Money => false,
            Resource::Awakeness | Resource::Satiety | Resource::Groceries => true,
            _ => unimplemented!(),
        }
    }

    fn importance(resource: Resource, time: TimeOfDay) -> f32 {
        let hour = time.hours_minutes().0;

        let bihourly_importance = match resource {
            Resource::Awakeness => Some([7, 7, 7, 7, 5, 5, 5, 5, 5, 5, 7, 7]),
            Resource::Satiety => Some([0, 0, 5, 5, 1, 5, 5, 1, 5, 5, 1, 1]),
            Resource::Money => Some([0, 0, 3, 3, 5, 5, 5, 3, 3, 1, 1, 1]),
            Resource::Groceries => Some([0, 0, 4, 4, 1, 4, 4, 4, 4, 4, 0, 0]),
            _ => None,
        };

        bihourly_importance
            .map(|lookup| lookup[hour / 2] as f32)
            .unwrap_or(0.0)
    }

    fn interesting_resources() -> &'static [Resource] {
        &[
            Resource::Awakeness,
            Resource::Satiety,
            Resource::Money,
            Resource::Groceries,
        ]
    }

    fn receive_deal(&mut self, deal: &Deal, member: MemberIdx, _: &mut World) {
        deal.delta.give_to_shared_private(
            &mut self.resources,
            &mut self.member_resources[member.0],
            Self::is_shared,
        );
    }

    fn provide_deal(&mut self, deal: &Deal, member: MemberIdx, _: &mut World) {
        let provide_awakeness = deal.delta.len() == 1 &&
            deal.delta.get(Resource::Awakeness).is_some();
        if !provide_awakeness {
            deal.delta.take_from_shared_private(
                &mut self.resources,
                &mut self.member_resources[member.0],
                Self::is_shared,
            );
        }
    }

    fn decay(&mut self, dt: Duration, _: &mut World) {
        for (i, member_resources) in self.member_resources.iter_mut().enumerate() {
            {
                let individuality = seed((self.id, i)).gen_range(0.8, 1.2);
                let awakeness = member_resources.mut_entry_or(Resource::Awakeness, 0.0);
                *awakeness -= 1.0 * individuality * dt.as_hours();
            }
            {
                let individuality = seed((self.id, i, 1u8)).gen_range(0.8, 1.2);
                let satiety = member_resources.mut_entry_or(Resource::Satiety, 0.0);
                if *satiety < 0.0 {
                    let groceries = self.resources.mut_entry_or(Resource::Groceries, 0.0);
                    *groceries -= 3.0;
                    *satiety += 3.0;
                }
                *satiety -= 1.0 * individuality * dt.as_hours();
            }
        }
    }

    fn task_succeeded(&mut self, member: MemberIdx, world: &mut World) {
        self.log.log("Task succeeded\n");
        if let TaskState::StartedAt(_, location) = self.member_tasks[member.0].state {
            self.stop_task(member, Some(location), world);
        } else {
            panic!("Can't finish unstarted task");
        }
    }

    fn task_failed(&mut self, member: MemberIdx, location: RoughLocationID, world: &mut World) {
        self.stop_task(member, Some(location), world);
    }

    fn reset_member_task(&mut self, member: MemberIdx, world: &mut World) {
        self.log.log(
            format!("Reset member {}\n", member.0).as_str(),
        );
        TaskEndSchedulerID::local_first(world).deschedule(self.id.into(), member, world);

        self.stop_task(member, None, world);
    }

    fn stop_using(&mut self, offer: OfferID, world: &mut World) {
        if let Some(Entry(associated_resource, _)) =
            self.used_offers
                .iter()
                .find(|&&Entry(_, resource_offer)| resource_offer == offer)
                .cloned()
        {
            self.used_offers.remove(associated_resource);
            offer.stopped_using(self.id.into(), None, world);
        }

        for (i, member_used_offers) in self.member_used_offers.iter_mut().enumerate() {
            if let Some(Entry(associated_resource, _)) =
                member_used_offers
                    .iter()
                    .find(|&&Entry(_, resource_offer)| resource_offer == offer)
                    .cloned()
            {
                member_used_offers.remove(associated_resource);
                offer.stopped_using(self.id.into(), Some(MemberIdx(i)), world);
            }
        }
    }

    fn destroy(&mut self, world: &mut World) {
        for &Entry(_, offer) in self.used_offers.iter() {
            offer.stopped_using(self.id.into(), None, world);
        }
        for (i, member_used_offers) in self.member_used_offers.iter().enumerate() {
            for &Entry(_, offer) in member_used_offers.iter() {
                offer.stopped_using(self.id.into(), Some(MemberIdx(i)), world);
            }
        }
        self.sleep_offer.withdraw_internal(world);
        self.home.remove_household(self.id.into(), world);
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
            ui.tree_node(im_str!("The {} Family:", family_name(self.id)))
                .build(|| {
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
                            ui.same_line(100.0);
                            let amount = self.resources.get(*resource).cloned().unwrap_or(0.0);
                            ui.text(im_str!("{:.2}", amount));
                        }
                    }
                    for (i, (member_resources, member_task)) in
                        self.member_resources
                            .iter()
                            .zip(&self.member_tasks)
                            .enumerate()
                    {
                        ui.spacing();
                        ui.text(im_str!(
                            "{}:",
                            member_name(self.id, MemberIdx(i)),
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
                                ui.same_line(100.0);
                                let amount =
                                    member_resources.get(*resource).cloned().unwrap_or(0.0);
                                ui.text(im_str!("{:.2}", amount));
                            }
                        }
                    }
                    ui.tree_node(im_str!("Log")).build(|| for line in self.log
                        .0
                        .lines()
                    {
                        ui.text(im_str!("{}", line));
                    });
                })
        });

        return_to.ui_drawn(ui, world);
    }
}

use core::simulation::TICKS_PER_SIM_SECOND;

impl Simulatable for Family {
    fn tick(&mut self, _dt: f32, current_instant: Instant, world: &mut World) {
        if (current_instant.ticks() + self.id._raw_id.instance_id as usize) %
            (UPDATE_EVERY_N_SECS * TICKS_PER_SIM_SECOND) == 0
        {
            self.decay(Duration(UPDATE_EVERY_N_SECS * TICKS_PER_SIM_SECOND), world);
        }
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.register::<Family>();
    auto_setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;
