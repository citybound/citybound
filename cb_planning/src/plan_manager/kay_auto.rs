//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;



impl<Logic: PlanningLogic + 'static> Actor for PlanManager<Logic> {
    type ID = PlanManagerID<Logic>;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Serialize, Deserialize)] #[serde(transparent)]
pub struct PlanManagerID<Logic: PlanningLogic + 'static> {
    _raw_id: RawID, _marker: ::std::marker::PhantomData<Box<(Logic)>>
}

impl<Logic: PlanningLogic + 'static> Copy for PlanManagerID<Logic> {}
impl<Logic: PlanningLogic + 'static> Clone for PlanManagerID<Logic> { fn clone(&self) -> Self { *self } }
impl<Logic: PlanningLogic + 'static> ::std::fmt::Debug for PlanManagerID<Logic> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "PlanManagerID<Logic>({:?})", self._raw_id)
    }
}
impl<Logic: PlanningLogic + 'static> ::std::hash::Hash for PlanManagerID<Logic> {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self._raw_id.hash(state);
    }
}
impl<Logic: PlanningLogic + 'static> PartialEq for PlanManagerID<Logic> {
    fn eq(&self, other: &PlanManagerID<Logic>) -> bool {
        self._raw_id == other._raw_id
    }
}
impl<Logic: PlanningLogic + 'static> Eq for PlanManagerID<Logic> {}

impl<Logic: PlanningLogic + 'static> TypedID for PlanManagerID<Logic> {
    type Target = PlanManager<Logic>;

    fn from_raw(id: RawID) -> Self {
        PlanManagerID { _raw_id: id, _marker: ::std::marker::PhantomData }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl<Logic: PlanningLogic + 'static> PlanManagerID<Logic> {
    pub fn spawn(world: &mut World) -> Self {
        let id = PlanManagerID::<Logic>::from_raw(world.allocate_instance_id::<PlanManager<Logic>>());
        let swarm = world.local_broadcast::<PlanManager<Logic>>();
        world.send(swarm, MSG_PlanManager_spawn::<Logic>(id, ));
        id
    }
    
    pub fn start_new_project(self, project_id: ProjectID, world: &mut World) {
        world.send(self.as_raw(), MSG_PlanManager_start_new_project(project_id));
    }
    
    pub fn implement(self, project_id: ProjectID, world: &mut World) {
        world.send(self.as_raw(), MSG_PlanManager_implement(project_id));
    }
    
    pub fn implement_artificial_project(self, project: Project < Logic :: GestureIntent >, based_on: CVec < PrototypeID >, world: &mut World) {
        world.send(self.as_raw(), MSG_PlanManager_implement_artificial_project::<Logic>(project, based_on));
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_PlanManager_spawn<Logic: PlanningLogic + 'static>(pub PlanManagerID<Logic>, );
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_PlanManager_start_new_project(pub ProjectID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_PlanManager_implement(pub ProjectID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_PlanManager_implement_artificial_project<Logic: PlanningLogic + 'static>(pub Project < Logic :: GestureIntent >, pub CVec < PrototypeID >);


#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup<Logic: PlanningLogic + 'static>(system: &mut ActorSystem) {
    
    
    system.add_spawner::<PlanManager<Logic>, _, _>(
        |&MSG_PlanManager_spawn::<Logic>(id, ), world| {
            PlanManager::<Logic>::spawn(id, world)
        }, false
    );
    
    system.add_handler::<PlanManager<Logic>, _, _>(
        |&MSG_PlanManager_start_new_project(project_id), instance, world| {
            instance.start_new_project(project_id, world); Fate::Live
        }, false
    );
    
    system.add_handler::<PlanManager<Logic>, _, _>(
        |&MSG_PlanManager_implement(project_id), instance, world| {
            instance.implement(project_id, world); Fate::Live
        }, false
    );
    
    system.add_handler::<PlanManager<Logic>, _, _>(
        |&MSG_PlanManager_implement_artificial_project::<Logic>(ref project, ref based_on), instance, world| {
            instance.implement_artificial_project(project, based_on, world); Fate::Live
        }, false
    );
}