//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;

#[derive(Serialize, Deserialize)] #[serde(transparent)]
pub struct PlanningUIID<Logic: PlanningLogic> {
    _raw_id: RawID, _marker: ::std::marker::PhantomData<Box<(Logic)>>
}

impl<Logic: PlanningLogic> Copy for PlanningUIID<Logic> {}
impl<Logic: PlanningLogic> Clone for PlanningUIID<Logic> { fn clone(&self) -> Self { *self } }
impl<Logic: PlanningLogic> ::std::fmt::Debug for PlanningUIID<Logic> {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "PlanningUIID<Logic>({:?})", self._raw_id)
    }
}
impl<Logic: PlanningLogic> ::std::hash::Hash for PlanningUIID<Logic> {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self._raw_id.hash(state);
    }
}
impl<Logic: PlanningLogic> PartialEq for PlanningUIID<Logic> {
    fn eq(&self, other: &PlanningUIID<Logic>) -> bool {
        self._raw_id == other._raw_id
    }
}
impl<Logic: PlanningLogic> Eq for PlanningUIID<Logic> {}

pub struct PlanningUIRepresentative<Logic: PlanningLogic>{ _marker: ::std::marker::PhantomData<Box<(Logic)>> }

impl<Logic: PlanningLogic> ActorOrActorTrait for PlanningUIRepresentative<Logic> {
    type ID = PlanningUIID<Logic>;
}

impl<Logic: PlanningLogic> TypedID for PlanningUIID<Logic> {
    type Target = PlanningUIRepresentative<Logic>;

    fn from_raw(id: RawID) -> Self {
        PlanningUIID { _raw_id: id, _marker: ::std::marker::PhantomData }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl<Logic: PlanningLogic, Act: Actor + PlanningUI<Logic>> TraitIDFrom<Act> for PlanningUIID<Logic> {}

impl<Logic: PlanningLogic> PlanningUIID<Logic> {
    pub fn on_plans_update(self, master_update: PlanHistoryUpdate < Logic :: GestureIntent >, project_updates: CHashMap < ProjectID , ProjectUpdate < Logic :: GestureIntent > >, world: &mut World) {
        world.send(self.as_raw(), MSG_PlanningUI_on_plans_update::<Logic>(master_update, project_updates));
    }
    
    pub fn on_project_preview_update(self, project_id: ProjectID, effective_history: PlanHistory < Logic :: GestureIntent >, result_update: PlanResultUpdate < Logic :: PrototypeKind >, new_actions: ActionGroups, world: &mut World) {
        world.send(self.as_raw(), MSG_PlanningUI_on_project_preview_update::<Logic>(project_id, effective_history, result_update, new_actions));
    }

    pub fn register_trait(system: &mut ActorSystem) {
        system.register_trait::<PlanningUIRepresentative<Logic>>();
        system.register_trait_message::<MSG_PlanningUI_on_plans_update<Logic>>();
        system.register_trait_message::<MSG_PlanningUI_on_project_preview_update<Logic>>();
    }

    pub fn register_implementor<Act: Actor + PlanningUI<Logic>>(system: &mut ActorSystem) {
        system.register_implementor::<Act, PlanningUIRepresentative<Logic>>();
        system.add_handler::<Act, _, _>(
            |&MSG_PlanningUI_on_plans_update::<Logic>(ref master_update, ref project_updates), instance, world| {
                instance.on_plans_update(master_update, project_updates, world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_PlanningUI_on_project_preview_update::<Logic>(project_id, ref effective_history, ref result_update, ref new_actions), instance, world| {
                instance.on_project_preview_update(project_id, effective_history, result_update, new_actions, world); Fate::Live
            }, false
        );
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_PlanningUI_on_plans_update<Logic: PlanningLogic>(pub PlanHistoryUpdate < Logic :: GestureIntent >, pub CHashMap < ProjectID , ProjectUpdate < Logic :: GestureIntent > >);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_PlanningUI_on_project_preview_update<Logic: PlanningLogic>(pub ProjectID, pub PlanHistory < Logic :: GestureIntent >, pub PlanResultUpdate < Logic :: PrototypeKind >, pub ActionGroups);



#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup<Logic: PlanningLogic>(system: &mut ActorSystem) {
    PlanningUIID::<Logic>::register_trait(system);
    
}