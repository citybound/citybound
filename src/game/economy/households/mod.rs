use kay::{ID, ActorSystem, World, Fate};
use kay::swarm::{Swarm, SubActor};
use compact::CVec;
use core::simulation::TimeOfDay;

use super::resources::{ResourceMap, ResourceId, ResourceAmount, Entry};
use ordered_float::OrderedFloat;

mod judgement_table;
use self::judgement_table::judgement_table;

mod tasks;
use self::tasks::Task;

mod buildings;

use super::market::{Deal, Offer, Evaluate, Find};

#[derive(Copy, Clone)]
pub struct MemberIdx(usize);

#[derive(Compact, Clone)]
enum DecisionState {
    None,
    Waiting,
    Deciding(MemberIdx, Deal, usize),
}

#[derive(Compact, Clone, SubActor)]
pub struct Family {
    _id: Option<ID>,
    resources: ResourceMap<ResourceAmount>,
    member_resources: CVec<ResourceMap<ResourceAmount>>,
    member_tasks: CVec<Task>,
    decision_state: DecisionState,
    used_offers: ResourceMap<ID>,
    member_used_offers: CVec<ResourceMap<ID>>,
}

const N_TOP_PROBLEMS: usize = 3;

use super::resources::r_properties;

impl Family {
    pub fn top_problems(&self, member: MemberIdx, time: TimeOfDay) -> Vec<(ResourceId, f32)> {
        let mut resource_graveness = self.resources
            .iter()
            .chain(self.member_resources[member.0].iter())
            .map(|&Entry(resource, amount)| {
                (resource, -amount * judgement_table().importance(resource, time))
            })
            .collect::<Vec<_>>();
        resource_graveness.sort_by_key(|&(_r, i)| OrderedFloat(i));

        resource_graveness.truncate(N_TOP_PROBLEMS);
        resource_graveness
    }

    pub fn find_new_task_for(&self,
                             member: MemberIdx,
                             time: TimeOfDay,
                             location: ID,
                             world: &mut World) {
        for (resource, graveness) in self.top_problems(member, time) {
            let maybe_offer = if r_properties(resource).supplier_shared {
                self.used_offers.get(resource)
            } else {
                self.member_used_offers[member.0].get(resource)
            };

            if let Some(&offer) = maybe_offer {
                world.send(offer, Evaluate { time, location, requester: self.id() });
            } else {
                world.send_to_id_of::<Swarm<Offer>, _>(Find {
                                                           time,
                                                           location,
                                                           resource,
                                                           requester: self.id(),
                                                       });
            }
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
                if let Some((idle_member_idx, location)) =
                    family
                        .member_tasks
                        .iter()
                        .enumerate()
                        .filter_map(|(idx, m)| match m.state {
                                        TaskState::IdleAt(loc) => Some((idx, loc)),
                                        _ => None,
                                    })
                        .next() {
                    family.find_new_task_for(MemberIdx(idle_member_idx),
                                             TimeOfDay::from_tick(current_tick),
                                             location,
                                             world);
                }
            };
            Fate::Live
        });
    }));
}

#[derive(Compact, Clone, SubActor)]
pub struct Company {
    _id: Option<ID>,
    resources: ResourceMap<ResourceAmount>,
    worker_tasks: CVec<Task>,
    used_offers: ResourceMap<ID>,
    own_offers: CVec<ID>,
}
