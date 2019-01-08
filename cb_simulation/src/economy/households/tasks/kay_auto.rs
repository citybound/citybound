//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;



impl Actor for TaskEndScheduler {
    type ID = TaskEndSchedulerID;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)] #[serde(transparent)]
pub struct TaskEndSchedulerID {
    _raw_id: RawID
}

impl TypedID for TaskEndSchedulerID {
    type Target = TaskEndScheduler;

    fn from_raw(id: RawID) -> Self {
        TaskEndSchedulerID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl TaskEndSchedulerID {
    pub fn spawn(world: &mut World) -> Self {
        let id = TaskEndSchedulerID::from_raw(world.allocate_instance_id::<TaskEndScheduler>());
        let swarm = world.local_broadcast::<TaskEndScheduler>();
        world.send(swarm, MSG_TaskEndScheduler_spawn(id, ));
        id
    }
    
    pub fn schedule(self, end: Instant, household: HouseholdID, member: MemberIdx, world: &mut World) {
        world.send(self.as_raw(), MSG_TaskEndScheduler_schedule(end, household, member));
    }
    
    pub fn deschedule(self, household: HouseholdID, member: MemberIdx, world: &mut World) {
        world.send(self.as_raw(), MSG_TaskEndScheduler_deschedule(household, member));
    }
}

#[derive(Copy, Clone)] #[allow(non_camel_case_types)]
struct MSG_TaskEndScheduler_spawn(pub TaskEndSchedulerID, );
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_TaskEndScheduler_schedule(pub Instant, pub HouseholdID, pub MemberIdx);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_TaskEndScheduler_deschedule(pub HouseholdID, pub MemberIdx);

impl Into<TemporalID> for TaskEndSchedulerID {
    fn into(self) -> TemporalID {
        TemporalID::from_raw(self.as_raw())
    }
}

#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    
    TemporalID::register_implementor::<TaskEndScheduler>(system);
    system.add_spawner::<TaskEndScheduler, _, _>(
        |&MSG_TaskEndScheduler_spawn(id, ), world| {
            TaskEndScheduler::spawn(id, world)
        }, false
    );
    
    system.add_handler::<TaskEndScheduler, _, _>(
        |&MSG_TaskEndScheduler_schedule(end, household, member), instance, world| {
            instance.schedule(end, household, member, world); Fate::Live
        }, false
    );
    
    system.add_handler::<TaskEndScheduler, _, _>(
        |&MSG_TaskEndScheduler_deschedule(household, member), instance, world| {
            instance.deschedule(household, member, world); Fate::Live
        }, false
    );
}