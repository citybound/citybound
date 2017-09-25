use kay::{ActorSystem, World};
use compact::CVec;
use core::simulation::{Timestamp, Seconds, Simulatable, SimulatableID, MSG_Simulatable_tick};
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

#[derive(Compact, Clone)]
pub struct TaskEndScheduler {
    id: TaskEndSchedulerID,
    task_ends: CVec<(Timestamp, HouseholdID, MemberIdx)>,
}

impl TaskEndScheduler {
    pub fn spawn(id: TaskEndSchedulerID, _: &mut World) -> TaskEndScheduler {
        TaskEndScheduler { id, task_ends: CVec::new() }
    }

    pub fn schedule(
        &mut self,
        end: Timestamp,
        household: HouseholdID,
        member: MemberIdx,
        _: &mut World,
    ) {
        let maybe_idx = self.task_ends.binary_search_by_key(
            &(end.iticks()),
            |&(e, _, _)| -(e.iticks()),
        );
        let insert_idx = match maybe_idx {
            Ok(idx) | Err(idx) => idx,
        };
        self.task_ends.insert(insert_idx, (end, household, member));
    }
}

impl Simulatable for TaskEndScheduler {
    fn tick(&mut self, _dt: f32, current_tick: Timestamp, world: &mut World) {
        while self.task_ends
            .last()
            .map(|&(end, _, _)| end < current_tick)
            .unwrap_or(false)
        {
            let (_, household, member) = self.task_ends.pop().expect(
                "just checked that there are WIP tasks",
            );
            household.task_succeeded(member, world);
        }
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.register::<TaskEndScheduler>();

    auto_setup(system);

    TaskEndSchedulerID::spawn(&mut system.world());
}

mod kay_auto;
pub use self::kay_auto::*;
