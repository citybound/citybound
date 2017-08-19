use kay::{ID, ActorSystem, Fate, World};
use kay::swarm::{Swarm, SubActor};
use core::simulation::{Tick, Timestamp, DurationSeconds, Simulation, WakeUpIn};
use transport::pathfinding::RoughDestinationID;
use transport::pathfinding::trip::TripID;
use super::super::resources::ResourceId;
use super::super::market::OfferID;

use super::MemberIdx;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum TaskState {
    GettingReadyAt(RoughDestinationID),
    InTrip(TripID),
    StartedAt(Timestamp, RoughDestinationID),
    IdleAt(RoughDestinationID),
}

#[derive(Copy, Clone)]
pub struct Task {
    pub goal: Option<(ResourceId, OfferID)>,
    pub duration: DurationSeconds,
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
    Failure {
        member: MemberIdx,
        location: RoughDestinationID,
    },
}

pub fn setup(system: &mut ActorSystem) {
    system.add(TaskEndScheduler::default(), |mut the_scheduler| {
        the_scheduler.on(|&ScheduleTaskEnd(end, family_id, member), scheduler, _| {
            let maybe_idx = scheduler.task_ends.binary_search_by_key(
                &(end.iticks()),
                |&(e, _, _)| -(e.iticks()),
            );
            let insert_idx = match maybe_idx {
                Ok(idx) | Err(idx) => idx,
            };
            scheduler.task_ends.insert(
                insert_idx,
                (end, family_id, member),
            );
            Fate::Live
        });

        the_scheduler.on(|&Tick { current_tick, .. }, scheduler, world| {
            while scheduler
                .task_ends
                .last()
                .map(|&(end, _, _)| end < current_tick)
                .unwrap_or(false)
            {
                let (_, family_id, member) = scheduler.task_ends.pop().expect(
                    "just checked that there are WIP tasks",
                );
                world.send(family_id, Complete::Success { member });
            }
            Fate::Live
        });
    });

    system.extend(Swarm::<super::Family>::subactors(|mut each_family| {
        each_family.on(move |result, family, world| {
            match *result {
                Complete::Success { member } => {
                    if let TaskState::StartedAt(_, location) = family.member_tasks[member.0].state {
                        family.stop_task(member, location, world)
                    } else {
                        panic!("Can't finish unstarted task");
                    }
                }
                Complete::Failure { member, location } => family.stop_task(member, location, world),
            };
            Fate::Live
        })
    }));

}

impl super::Family {
    pub fn start_task(
        &mut self,
        member: MemberIdx,
        start: Timestamp,
        location: RoughDestinationID,
        world: &mut World,
    ) {
        world.send_to_id_of::<TaskEndScheduler, _>(ScheduleTaskEnd(
            start + self.member_tasks[member.0].duration,
            self.id(),
            member,
        ));
        self.member_tasks[member.0].state = TaskState::StartedAt(start, location);
    }

    pub fn stop_task(
        &mut self,
        member: MemberIdx,
        location: RoughDestinationID,
        world: &mut World,
    ) {
        self.member_tasks[member.0].state = TaskState::IdleAt(location);
        world.send_to_id_of::<Simulation, _>(
            WakeUpIn(DurationSeconds::new(0).into(), self.id.into()),
        );
    }
}
