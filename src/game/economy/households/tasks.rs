use kay::{ID, ActorSystem, Fate};
use kay::swarm::{Swarm, SubActor};
use core::simulation::{Tick, Timestamp, Duration, Simulation, WakeUpIn};

use super::MemberIdx;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum TaskState {
    InTrip(ID),
    DoingFromToAt(usize, usize, ID),
    IdleAt(ID),
}

#[derive(Copy, Clone)]
pub struct Task {
    pub offer: ID,
    pub state: TaskState,
}

#[derive(Default)]
pub struct TaskEndScheduler {
    task_ends: Vec<(Timestamp, ID, MemberIdx)>,
}

#[derive(Compact, Clone)]
pub struct ScheduleTaskEnd(Timestamp, ID, MemberIdx);

#[derive(Copy, Clone)]
pub enum Complete {
    Success { member: MemberIdx },
    Failure { member: MemberIdx, location: ID },
}

pub fn setup(system: &mut ActorSystem) {
    system.add(TaskEndScheduler::default(), |mut the_scheduler| {
        the_scheduler.on(|&ScheduleTaskEnd(end, task_id, member), scheduler, _| {
            let maybe_idx =
                scheduler
                    .task_ends
                    .binary_search_by_key(&(end.0 as isize), |&(e, _, _)| -(e.0 as isize));
            let insert_idx = match maybe_idx {
                Ok(idx) | Err(idx) => idx,
            };
            scheduler
                .task_ends
                .insert(insert_idx, (end, task_id, member));
            Fate::Live
        });

        the_scheduler.on(|&Tick { current_tick, .. }, scheduler, world| {
            while scheduler
                      .task_ends
                      .last()
                      .map(|&(end, _, _)| end.0 < current_tick.0)
                      .unwrap_or(false) {
                let (_, id, member) = scheduler
                    .task_ends
                    .pop()
                    .expect("just checked that there are WIP tasks");
                world.send(id, Complete::Success { member });
            }
            Fate::Live
        });
    });

    system.extend(Swarm::<super::Family>::subactors(|mut each_family| {
        let sim_id = each_family.world().id::<Simulation>();
        each_family.on(move |result, family, world| {
            match *result {
                Complete::Success { member } => {
                    if let TaskState::DoingFromToAt(_, _, location) =
                        family.member_tasks[member.0].state {
                        family.member_tasks[member.0].state = TaskState::IdleAt(location);
                        world.send(sim_id, WakeUpIn(Duration(0), family.id()))
                    } else {
                        panic!("Can't finish unstarted task");
                    }
                }
                Complete::Failure { member, location } => {
                    family.member_tasks[member.0].state = TaskState::IdleAt(location);
                    world.send(sim_id, WakeUpIn(Duration(0), family.id()))
                }
            };
            Fate::Live
        })
    }));

}