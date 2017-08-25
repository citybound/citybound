use kay::{ActorSystem, Fate};
use core::simulation::{Tick, Timestamp, Seconds};
use transport::pathfinding::RoughLocationID;
use transport::pathfinding::trip::TripID;
use super::super::resources::ResourceId;
use super::super::market::OfferID;

use super::{HouseholdID, MemberIdx};

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum TaskState {
    GettingReadyAt(RoughLocationID),
    InTrip(TripID),
    StartedAt(Timestamp, RoughLocationID),
    IdleAt(RoughLocationID),
}

#[derive(Copy, Clone)]
pub struct Task {
    pub goal: Option<(ResourceId, OfferID)>,
    pub duration: Seconds,
    pub state: TaskState,
}

impl Task {
    pub fn idle_at(location: RoughLocationID) -> Self {
        Task {
            goal: None,
            duration: Seconds(0),
            state: TaskState::IdleAt(location),
        }
    }
}

#[derive(Default)]
pub struct TaskEndScheduler {
    task_ends: Vec<(Timestamp, HouseholdID, MemberIdx)>,
}

#[derive(Compact, Clone)]
pub struct ScheduleTaskEnd(pub Timestamp, pub HouseholdID, pub MemberIdx);

pub fn setup(system: &mut ActorSystem) {
    system.add(TaskEndScheduler::default(), |mut the_scheduler| {
        the_scheduler.on(|&ScheduleTaskEnd(end, household, member), scheduler, _| {
            let maybe_idx = scheduler.task_ends.binary_search_by_key(
                &(end.iticks()),
                |&(e, _, _)| -(e.iticks()),
            );
            let insert_idx = match maybe_idx {
                Ok(idx) | Err(idx) => idx,
            };
            scheduler.task_ends.insert(
                insert_idx,
                (end, household, member),
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
                let (_, household, member) = scheduler.task_ends.pop().expect(
                    "just checked that there are WIP tasks",
                );
                household.task_succeeded(member, world);
            }
            Fate::Live
        });
    });
}
