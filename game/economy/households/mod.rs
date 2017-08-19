use kay::{ActorSystem, World, Fate};
use kay::swarm::{Swarm, SubActor};
use compact::{CVec, CDict};
use core::simulation::{TimeOfDay, Timestamp};

use super::resources::{ResourceMap, ResourceId, ResourceAmount, Entry};
use ordered_float::OrderedFloat;

mod judgement_table;
use self::judgement_table::judgement_table;

mod tasks;
use self::tasks::Task;

use super::market::{Deal, Market, OfferID, Search, EvaluatedDeal, EvaluationRequester,
                    EvaluationRequesterID, MSG_EvaluationRequester_on_result,
                    EvaluatedSearchResult};
use super::buildings::BuildingID;
use transport::pathfinding::trip::{TripListenerID, MSG_TripListener_trip_created,
                                   MSG_TripListener_trip_result};
use transport::pathfinding::RoughDestinationID;

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct MemberIdx(usize);

use imgui::Ui;
use kay::{External, ID};
use stagemaster::Ui2dDrawn;

pub trait Household {
    fn on_applicable_deal(&mut self, deal: &Deal, member: MemberIdx, world: &mut World);
    fn inspect(&mut self, imgui_ui: &External<Ui<'static>>, return_to: ID, world: &mut World);
}

#[derive(Compact, Clone)]
struct DecisionResourceEntry {
    n_deals_expected: usize,
    deals: CVec<EvaluatedDeal>,
}

#[derive(Compact, Clone)]
enum DecisionState {
    None,
    Choosing(MemberIdx, Timestamp, CDict<ResourceId, DecisionResourceEntry>),
    WaitingForTrip(MemberIdx),
}

#[derive(Compact, Clone)]
pub struct Family {
    id: FamilyID,
    home: BuildingID,
    resources: ResourceMap<ResourceAmount>,
    member_resources: CVec<ResourceMap<ResourceAmount>>,
    member_tasks: CVec<Task>,
    decision_state: DecisionState,
    used_offers: ResourceMap<OfferID>,
    member_used_offers: CVec<ResourceMap<OfferID>>,
}

const N_TOP_PROBLEMS: usize = 3;

use super::resources::r_properties;

fn resource_graveness_helper(resource: ResourceId, amount: ResourceAmount, time: TimeOfDay) -> f32 {
    -amount * judgement_table().importance(resource, time)
}

use core::simulation::DurationSeconds;

impl Family {
    pub fn move_into(id: FamilyID, n_members: usize, home: BuildingID, _: &mut World) -> Family {
        Family {
            id,
            home,
            resources: ResourceMap::new(),
            member_resources: (0..n_members)
                .into_iter()
                .map(|_| ResourceMap::new())
                .collect(),
            member_tasks: (0..n_members)
                .into_iter()
                .map(|_| {
                    Task {
                        goal: None,
                        duration: DurationSeconds::new(0),
                        state: TaskState::IdleAt(home.into()),
                    }
                })
                .collect(),
            decision_state: DecisionState::None,
            used_offers: ResourceMap::new(),
            member_used_offers: (0..n_members)
                .into_iter()
                .map(|_| ResourceMap::new())
                .collect(),
        }
    }

    pub fn top_problems(&self, member: MemberIdx, time: TimeOfDay) -> Vec<(ResourceId, f32)> {
        let mut resource_graveness = self.resources
            .iter()
            .chain(self.member_resources[member.0].iter())
            .map(|&Entry(resource, amount)| {
                (resource, resource_graveness_helper(resource, amount, time))
            })
            .collect::<Vec<_>>();
        resource_graveness.sort_by_key(|&(_r, i)| OrderedFloat(i));

        resource_graveness.truncate(N_TOP_PROBLEMS);
        resource_graveness
    }

    pub fn find_new_task_for(
        &mut self,
        member: MemberIdx,
        tick: Timestamp,
        location: RoughDestinationID,
        world: &mut World,
    ) {
        let mut decision_entries = CDict::<ResourceId, DecisionResourceEntry>::new();
        let time = TimeOfDay::from_tick(tick);

        for (resource, _) in self.top_problems(member, time) {
            let maybe_offer = if r_properties(resource).supplier_shared {
                self.used_offers.get(resource)
            } else {
                self.member_used_offers[member.0].get(resource)
            };

            if let Some(&offer) = maybe_offer {
                offer.evaluate(tick, location, self.id.into(), world);
            } else {
                world.send_to_id_of::<Market, _>(Search {
                    time,
                    location,
                    resource,
                    requester: self.id(),
                });
            }

            decision_entries.insert(
                resource,
                DecisionResourceEntry { n_deals_expected: 1, deals: CVec::new() },
            );
        }

        self.decision_state = DecisionState::Choosing(member, tick, decision_entries);
    }

    pub fn choose_deal(&mut self, world: &mut World) {
        let (member, tick, best_offer) =
            if let DecisionState::Choosing(member, tick, ref entries) = self.decision_state {
                let time = TimeOfDay::from_tick(tick);
                let maybe_best = entries
                    .values()
                    .flat_map(|entry| {
                        entry
                            .deals
                            .iter()
                            .filter(|evaluated| evaluated.from < time && evaluated.to > time)
                            .map(|evaluated| {
                                let give_alleviation = resource_graveness_helper(
                                    evaluated.deal.give.0,
                                    -evaluated.deal.give.1,
                                    time,
                                );
                                let take_graveness: f32 = evaluated
                                    .deal
                                    .take
                                    .iter()
                                    .map(|&Entry(resource, amount)| {
                                        resource_graveness_helper(resource, -amount, time)
                                    })
                                    .sum();

                                let usefulness = give_alleviation /
                                    (take_graveness * evaluated.deal.duration.seconds() as f32);

                                (usefulness, evaluated)
                            })
                    })
                    .collect::<Vec<_>>()
                    .into_iter()
                    .max_by_key(|&(u, _e)| OrderedFloat(u));

                let best = maybe_best
                    .expect("TODO: deal with no suitable offer at all")
                    .1;
                let task = &mut self.member_tasks[member.0];

                *task = if let TaskState::IdleAt(loc) = task.state {
                    Task {
                        goal: Some((best.deal.give.0, best.offer)),
                        duration: best.deal.duration,
                        state: TaskState::GettingReadyAt(loc),
                    }
                } else {
                    panic!("Member who gets new task should be idle");
                };

                (member, tick, best.offer)
            } else {
                panic!("Tried to choose deal while not deciding");
            };

        self.decision_state = DecisionState::WaitingForTrip(member);
        best_offer.get_applicable_deal(self.id.into(), member, world);
        self.start_trip(member, tick, world);
    }

    pub fn start_trip(&mut self, member: MemberIdx, tick: Timestamp, world: &mut World) {
        if let Task {
            goal: Some((_, offer)),
            state: TaskState::GettingReadyAt(source),
            ..
        } = self.member_tasks[member.0]
        {
            TripID::spawn(source, offer.into(), Some(self.id.into()), tick, world);
        } else {
            panic!("Member should be getting ready before starting trip");
        }
    }
}

impl EvaluationRequester for Family {
    fn on_result(&mut self, result: &EvaluatedSearchResult, world: &mut World) {
        let &EvaluatedSearchResult { resource, n_to_expect, ref some_results } = result;

        let done = if let DecisionState::Choosing(_, _, ref mut entries) = self.decision_state {
            {
                let entry = entries.get_mut(resource).expect(
                    "Should have an entry for queried resource",
                );
                entry.n_deals_expected = n_to_expect;
                entry.deals.extend(some_results.clone());
            }

            entries.values().all(|entry| {
                entry.n_deals_expected == entry.deals.len()
            })
        } else {
            panic!("Received unexpected deal");
        };
        if done {
            self.choose_deal(world);
        }
    }
}

use economy::resources::{all_resource_ids, r_info};

impl Household for Family {
    fn on_applicable_deal(&mut self, deal: &Deal, member: MemberIdx, _: &mut World) {
        let resource_deltas = deal.take
            .iter()
            .map(|&Entry(resource, amount)| (resource, -amount))
            .chain(Some(deal.give));
        for (resource, delta) in resource_deltas {
            let resources = if r_properties(resource).ownership_shared {
                &mut self.resources
            } else {
                &mut self.member_resources[member.0]
            };
            *resources.mut_entry_or(resource, 0.0) += delta;
        }
    }

    fn inspect(&mut self, imgui_ui: &External<Ui<'static>>, return_to: ID, world: &mut World) {
        let ui = imgui_ui.steal();

        ui.window(im_str!("Building")).build(|| {
            ui.tree_node(im_str!("Household ID: {:?}", self.id._raw_id))
                .build(|| {
                    ui.tree_node(im_str!("Shared")).build(|| {
                        ui.text(im_str!("State"));
                        ui.same_line(150.0);
                        ui.text(im_str!(
                            "{}",
                            match self.decision_state {
                                DecisionState::None => "None",
                                DecisionState::Choosing(_, _, _) => "Waiting for choice",
                                DecisionState::WaitingForTrip(_) => "Waiting for trip",
                            }
                        ));

                        for resource in all_resource_ids() {
                            if r_properties(resource).ownership_shared {
                                ui.text(im_str!("{}", r_info(resource).0));
                                ui.same_line(150.0);
                                let amount = self.resources.get(resource).cloned().unwrap_or(0.0);
                                ui.text(im_str!("{}", amount));
                            }
                        }
                    });
                    for (i, (member_resources, member_task)) in
                        self.member_resources
                            .iter()
                            .zip(&self.member_tasks)
                            .enumerate()
                    {
                        ui.tree_node(im_str!("Member #{}", i)).build(|| {
                            ui.text(im_str!("Task"));
                            ui.same_line(150.0);
                            ui.text(im_str!(
                                "{}",
                                match member_task.state {
                                    TaskState::IdleAt(_) => "Idle",
                                    TaskState::GettingReadyAt(_) => "Getting ready",
                                    TaskState::InTrip(_) => "In trip",
                                    TaskState::StartedAt(_, _) => "Started",
                                }
                            ));

                            for resource in all_resource_ids() {
                                if !r_properties(resource).ownership_shared {
                                    ui.text(im_str!("{}", r_info(resource).0));
                                    ui.same_line(150.0);
                                    let amount =
                                        member_resources.get(resource).cloned().unwrap_or(0.0);
                                    ui.text(im_str!("{}", amount));
                                }
                            }
                        });
                    }
                })
        });

        world.send(return_to, Ui2dDrawn { imgui_ui: ui });
    }
}

use core::simulation::{Sleeper, SleeperID, MSG_Sleeper_wake};

impl Sleeper for Family {
    fn wake(&mut self, current_tick: Timestamp, world: &mut World) {
        if let DecisionState::None = self.decision_state {
            let maybe_idle_idx_loc = self.member_tasks
                .iter()
                .enumerate()
                .filter_map(|(idx, m)| match m.state {
                    TaskState::IdleAt(loc) => Some((idx, loc)),
                    _ => None,
                })
                .next();
            if let Some((idle_member_idx, location)) = maybe_idle_idx_loc {
                self.find_new_task_for(MemberIdx(idle_member_idx), current_tick, location, world);
            }
        };
    }
}

use transport::pathfinding::trip::{TripListener, TripID};
use self::tasks::TaskState;

impl TripListener for Family {
    fn trip_created(&mut self, trip: TripID, _: &mut World) {
        self.decision_state = if let DecisionState::WaitingForTrip(member) = self.decision_state {
            self.member_tasks[member.0].state = TaskState::InTrip(trip);
            DecisionState::None
        } else {
            panic!("Should be in waiting for trip state")
        };
    }

    fn trip_result(
        &mut self,
        trip: TripID,
        location: RoughDestinationID,
        failed: bool,
        tick: Timestamp,
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
            let shared = r_properties(matching_resource).supplier_shared;
            let used_offers = if shared {
                &mut self.used_offers
            } else {
                &mut self.member_used_offers[matching_task_member.0]
            };

            let maybe_member = if shared {
                Some(matching_task_member)
            } else {
                None
            };

            if failed {
                used_offers.remove(matching_resource);
                matching_offer.stopped_using(self.id.into(), maybe_member, world);
            } else {
                used_offers.insert(matching_resource, matching_offer);
                matching_offer.started_using(self.id.into(), maybe_member, world);
            }
        }

        if failed {
            self.stop_task(matching_task_member, location, world);
        } else {
            self.start_task(matching_task_member, tick, location, world);
        }
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.add(Swarm::<Family>::new(), |_| {});

    auto_setup(system);
}

// #[derive(Compact, Clone)]
// pub struct Company {
//     id: CompanyID,
//     site: ID,
//     resources: ResourceMap<ResourceAmount>,
//     worker_tasks: CVec<Task>,
//     used_offers: ResourceMap<ID>,
//     own_offers: CVec<ID>,
// }


mod kay_auto;
pub use self::kay_auto::*;
