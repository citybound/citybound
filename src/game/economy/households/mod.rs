use kay::{ID, ActorSystem, World, Fate};
use kay::swarm::{Swarm, SubActor, CreateWith};
use compact::{CVec, CDict};
use core::simulation::{TimeOfDay, Timestamp};

use super::resources::{ResourceMap, ResourceId, ResourceAmount, Entry};
use ordered_float::OrderedFloat;

mod judgement_table;
use self::judgement_table::judgement_table;

mod tasks;
use self::tasks::Task;

mod buildings;

use super::market::{Market, Evaluate, Search, EvaluatedDeal, EvaluatedSearchResult,
                    GetApplicableDeal, ApplicableDeal, StartedUsing, StoppedUsing};
use super::super::lanes_and_cars::pathfinding::trip::{Trip, Start, CreatedTrip, TripResult};

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct MemberIdx(usize);

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

#[derive(Compact, Clone, SubActor)]
pub struct Family {
    _id: Option<ID>,
    home: ID,
    resources: ResourceMap<ResourceAmount>,
    member_resources: CVec<ResourceMap<ResourceAmount>>,
    member_tasks: CVec<Task>,
    decision_state: DecisionState,
    used_offers: ResourceMap<ID>,
    member_used_offers: CVec<ResourceMap<ID>>,
}

const N_TOP_PROBLEMS: usize = 3;

use super::resources::r_properties;

fn resource_graveness_helper(resource: ResourceId, amount: ResourceAmount, time: TimeOfDay) -> f32 {
    -amount * judgement_table().importance(resource, time)
}

impl Family {
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

    pub fn find_new_task_for(&mut self,
                             member: MemberIdx,
                             tick: Timestamp,
                             location: ID,
                             world: &mut World) {
        let mut decision_entries = CDict::<ResourceId, DecisionResourceEntry>::new();
        let time = TimeOfDay::from_tick(tick);

        for (resource, _) in self.top_problems(member, time) {
            let maybe_offer = if r_properties(resource).supplier_shared {
                self.used_offers.get(resource)
            } else {
                self.member_used_offers[member.0].get(resource)
            };

            if let Some(&offer) = maybe_offer {
                world.send(offer, Evaluate { time, location, requester: self.id() });
            } else {
                world.send_to_id_of::<Market, _>(Search {
                                                     time,
                                                     location,
                                                     resource,
                                                     requester: self.id(),
                                                 });
            }

            decision_entries.insert(resource,
                                    DecisionResourceEntry {
                                        n_deals_expected: 1,
                                        deals: CVec::new(),
                                    });
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
                                let give_alleviation =
                                    resource_graveness_helper(evaluated.deal.give.0,
                                                              -evaluated.deal.give.1,
                                                              time);
                                let take_graveness: f32 = evaluated
                                    .deal
                                    .take
                                    .iter()
                                    .map(|&Entry(resource, amount)| {
                                        resource_graveness_helper(resource, -amount, time)
                                    })
                                    .sum();

                                let usefulness = give_alleviation /
                                                 (take_graveness *
                                                  evaluated.deal.duration.seconds() as f32);

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
                        offer: best.offer,
                        goal: best.deal.give.0,
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
        world.send(best_offer, GetApplicableDeal(self.id(), member));
        self.start_trip(member, tick, world);
    }

    pub fn start_trip(&mut self, member: MemberIdx, tick: Timestamp, world: &mut World) {
        if let Task {
                   offer,
                   state: TaskState::GettingReadyAt(source),
                   ..
               } = self.member_tasks[member.0] {
            world.send_to_id_of::<Swarm<Trip>, _>(CreateWith(Trip::new(source,
                                                                       offer,
                                                                       Some(self.id())),
                                                             Start(tick)));
        } else {
            panic!("Member should be getting ready before starting trip");
        }
    }
}

use core::simulation::Wake;
use self::tasks::TaskState;

pub fn setup(system: &mut ActorSystem) {
    system.add(Swarm::<Family>::new(),
               Swarm::<Family>::subactors(|mut each_family| {
        each_family.on(|&Wake { current_tick }, family, world| {
            if let DecisionState::None = family.decision_state {
                let maybe_idle_idx_loc = family
                    .member_tasks
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, m)| match m.state {
                                    TaskState::IdleAt(loc) => Some((idx, loc)),
                                    _ => None,
                                })
                    .next();
                if let Some((idle_member_idx, location)) = maybe_idle_idx_loc {
                    family.find_new_task_for(MemberIdx(idle_member_idx),
                                             current_tick,
                                             location,
                                             world);
                }
            };
            Fate::Live
        });

        each_family.on(|&EvaluatedSearchResult { resource, n_to_expect, ref some_results },
                        family,
                        world| {
            let done = if let DecisionState::Choosing(_, _, ref mut entries) =
                family.decision_state {
                {
                    let entry = entries
                        .get_mut(resource)
                        .expect("Should have an entry for queried resource");
                    entry.n_deals_expected = n_to_expect;
                    entry.deals.extend(some_results.clone());
                }

                entries
                    .values()
                    .all(|entry| entry.n_deals_expected == entry.deals.len())
            } else {
                panic!("Received unexpected deal");
            };
            if done {
                family.choose_deal(world);
            }
            Fate::Live
        });

        each_family.on(|&CreatedTrip(trip), family, _| {
            family.decision_state = if let DecisionState::WaitingForTrip(member) =
                family.decision_state {
                family.member_tasks[member.0].state = TaskState::InTrip(trip);
                DecisionState::None
            } else {
                panic!("Should be in waiting for trip state")
            };
            Fate::Live
        });

        each_family.on(move |&TripResult { id, location, tick, failed }, family, world| {
            let (matching_task_member, matching_resource, matching_offer) =
                family
                    .member_tasks
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, task)| if let TaskState::InTrip(trip_id) = task.state {
                                    if trip_id == id {
                                        Some((MemberIdx(idx), task.goal, task.offer))
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                })
                    .next()
                    .expect("Should have a matching task");
            {
                let family_id = family.id();
                let shared = r_properties(matching_resource).supplier_shared;
                let used_offers = if shared {
                    &mut family.used_offers
                } else {
                    &mut family.member_used_offers[matching_task_member.0]
                };

                let maybe_member = if shared {
                    Some(matching_task_member)
                } else {
                    None
                };

                if failed {
                    used_offers.remove(matching_resource);
                    world.send(matching_offer, StoppedUsing(family_id, maybe_member));
                } else {
                    used_offers.insert(matching_resource, matching_offer);
                    world.send(matching_offer, StartedUsing(family_id, maybe_member));
                }
            }

            if failed {
                family.stop_task(matching_task_member, location, world);
            } else {
                family.start_task(matching_task_member, tick, location, world);
            }
            Fate::Live
        });

        each_family.on(|&ApplicableDeal(ref deal, member), family, _| {
            let resource_deltas = deal.take
                .iter()
                .map(|&Entry(resource, amount)| (resource, -amount))
                .chain(Some(deal.give));
            for (resource, delta) in resource_deltas {
                let resources = if r_properties(resource).ownership_shared {
                    &mut family.resources
                } else {
                    &mut family.member_resources[member.0]
                };
                *resources.mut_entry_or(resource, 0.0) += delta;
            }
            Fate::Live
        });
    }));
}

#[derive(Compact, Clone, SubActor)]
pub struct Company {
    _id: Option<ID>,
    site: ID,
    resources: ResourceMap<ResourceAmount>,
    worker_tasks: CVec<Task>,
    used_offers: ResourceMap<ID>,
    own_offers: CVec<ID>,
}
