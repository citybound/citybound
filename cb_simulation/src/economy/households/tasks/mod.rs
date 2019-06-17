use kay::{ActorSystem, World};
use compact::CVec;
use cb_time::actors::{Temporal, TemporalID};
use cb_time::units::{Instant, Duration};
use transport::pathfinding::RoughLocationID;
use transport::pathfinding::trip::TripID;
use super::super::resources::Resource;
use super::OfferID;

use super::{HouseholdID, MemberIdx};

#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum TaskState {
    GettingReadyAt(RoughLocationID),
    InTrip(TripID),
    StartedAt(Instant, RoughLocationID),
    IdleAt(RoughLocationID),
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct Task {
    pub goal: Option<(Resource, OfferID)>,
    pub duration: Duration,
    pub state: TaskState,
}

impl Task {
    pub fn idle_at(location: RoughLocationID) -> Self {
        Task {
            goal: None,
            duration: Duration(0),
            state: TaskState::IdleAt(location),
        }
    }
}

#[derive(Compact, Clone)]
pub struct TaskEndScheduler {
    id: TaskEndSchedulerID,
    task_ends: CVec<(Instant, HouseholdID, MemberIdx)>,
}

impl TaskEndScheduler {
    pub fn spawn(id: TaskEndSchedulerID, _: &mut World) -> TaskEndScheduler {
        TaskEndScheduler {
            id,
            task_ends: CVec::new(),
        }
    }

    pub fn schedule(
        &mut self,
        end: Instant,
        household: HouseholdID,
        member: MemberIdx,
        _: &mut World,
    ) {
        let maybe_idx = self
            .task_ends
            .binary_search_by_key(&(-end.iticks()), |&(e, ..)| -(e.iticks()));
        let insert_idx = match maybe_idx {
            Ok(idx) | Err(idx) => idx,
        };
        self.task_ends.insert(insert_idx, (end, household, member));
    }

    pub fn deschedule(&mut self, household: HouseholdID, member: MemberIdx, _: &mut World) {
        self.task_ends.retain(|&(_, task_household, task_member)| {
            task_household != household || task_member != member
        });
    }
}

impl Temporal for TaskEndScheduler {
    fn tick(&mut self, _dt: f32, current_instant: Instant, world: &mut World) {
        while self
            .task_ends
            .last()
            .map(|&(end, ..)| end < current_instant)
            .unwrap_or(false)
        {
            let (_, household, member) = self
                .task_ends
                .pop()
                .expect("just checked that there are WIP tasks");
            household.task_succeeded(member, world);
        }
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.register::<TaskEndScheduler>();
    auto_setup(system);
}

pub fn spawn(world: &mut World) {
    TaskEndSchedulerID::spawn(world);
}

mod kay_auto;
pub use self::kay_auto::*;
