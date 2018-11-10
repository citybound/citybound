//! This is all auto-generated. Do not touch.
#![cfg_attr(rustfmt, rustfmt_skip)]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;



impl Actor for PlanManager {
    type ID = PlanManagerID;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)] #[serde(transparent)]
pub struct PlanManagerID {
    _raw_id: RawID
}

impl TypedID for PlanManagerID {
    type Target = PlanManager;

    fn from_raw(id: RawID) -> Self {
        PlanManagerID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl PlanManagerID {
    pub fn spawn(initial_proposal_id: ProposalID, world: &mut World) -> Self {
        let id = PlanManagerID::from_raw(world.allocate_instance_id::<PlanManager>());
        let swarm = world.local_broadcast::<PlanManager>();
        world.send(swarm, MSG_PlanManager_spawn(id, initial_proposal_id));
        id
    }
    
    pub fn implement(&self, proposal_id: ProposalID, world: &mut World) {
        world.send(self.as_raw(), MSG_PlanManager_implement(proposal_id));
    }
    
    pub fn implement_artificial_proposal(&self, proposal: Proposal, based_on: CVec < PrototypeID >, world: &mut World) {
        world.send(self.as_raw(), MSG_PlanManager_implement_artificial_proposal(proposal, based_on));
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_PlanManager_spawn(pub PlanManagerID, pub ProposalID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_PlanManager_implement(pub ProposalID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_PlanManager_implement_artificial_proposal(pub Proposal, pub CVec < PrototypeID >);


#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    
    
    system.add_spawner::<PlanManager, _, _>(
        |&MSG_PlanManager_spawn(id, initial_proposal_id), world| {
            PlanManager::spawn(id, initial_proposal_id, world)
        }, false
    );
    
    system.add_handler::<PlanManager, _, _>(
        |&MSG_PlanManager_implement(proposal_id), instance, world| {
            instance.implement(proposal_id, world); Fate::Live
        }, false
    );
    
    system.add_handler::<PlanManager, _, _>(
        |&MSG_PlanManager_implement_artificial_proposal(ref proposal, ref based_on), instance, world| {
            instance.implement_artificial_proposal(proposal, based_on, world); Fate::Live
        }, false
    );
}