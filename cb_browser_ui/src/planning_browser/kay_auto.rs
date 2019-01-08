//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;



impl Actor for BrowserPlanningUI {
    type ID = BrowserPlanningUIID;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)] #[serde(transparent)]
pub struct BrowserPlanningUIID {
    _raw_id: RawID
}

impl TypedID for BrowserPlanningUIID {
    type Target = BrowserPlanningUI;

    fn from_raw(id: RawID) -> Self {
        BrowserPlanningUIID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl BrowserPlanningUIID {
    pub fn spawn(world: &mut World) -> Self {
        let id = BrowserPlanningUIID::from_raw(world.allocate_instance_id::<BrowserPlanningUI>());
        let swarm = world.local_broadcast::<BrowserPlanningUI>();
        world.send(swarm, MSG_BrowserPlanningUI_spawn(id, ));
        id
    }
}

#[derive(Copy, Clone)] #[allow(non_camel_case_types)]
struct MSG_BrowserPlanningUI_spawn(pub BrowserPlanningUIID, );

impl Into<FrameListenerID> for BrowserPlanningUIID {
    fn into(self) -> FrameListenerID {
        FrameListenerID::from_raw(self.as_raw())
    }
}

impl Into<PlanningUIID> for BrowserPlanningUIID {
    fn into(self) -> PlanningUIID {
        PlanningUIID::from_raw(self.as_raw())
    }
}

#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    
    FrameListenerID::register_implementor::<BrowserPlanningUI>(system);
    PlanningUIID::register_implementor::<BrowserPlanningUI>(system);
    system.add_spawner::<BrowserPlanningUI, _, _>(
        |&MSG_BrowserPlanningUI_spawn(id, ), world| {
            BrowserPlanningUI::spawn(id, world)
        }, false
    );
}