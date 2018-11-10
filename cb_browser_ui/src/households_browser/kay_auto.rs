//! This is all auto-generated. Do not touch.
#![cfg_attr(rustfmt, rustfmt_skip)]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;



impl Actor for BrowserHouseholdUI {
    type ID = BrowserHouseholdUIID;

    fn id(&self) -> Self::ID {
        self.id
    }
    unsafe fn set_id(&mut self, id: RawID) {
        self.id = Self::ID::from_raw(id);
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)] #[serde(transparent)]
pub struct BrowserHouseholdUIID {
    _raw_id: RawID
}

impl TypedID for BrowserHouseholdUIID {
    type Target = BrowserHouseholdUI;

    fn from_raw(id: RawID) -> Self {
        BrowserHouseholdUIID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl BrowserHouseholdUIID {
    pub fn spawn(world: &mut World) -> Self {
        let id = BrowserHouseholdUIID::from_raw(world.allocate_instance_id::<BrowserHouseholdUI>());
        let swarm = world.local_broadcast::<BrowserHouseholdUI>();
        world.send(swarm, MSG_BrowserHouseholdUI_spawn(id, ));
        id
    }
}

#[derive(Copy, Clone)] #[allow(non_camel_case_types)]
struct MSG_BrowserHouseholdUI_spawn(pub BrowserHouseholdUIID, );

impl Into<HouseholdUIID> for BrowserHouseholdUIID {
    fn into(self) -> HouseholdUIID {
        HouseholdUIID::from_raw(self.as_raw())
    }
}

#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    
    HouseholdUIID::register_implementor::<BrowserHouseholdUI>(system);
    system.add_spawner::<BrowserHouseholdUI, _, _>(
        |&MSG_BrowserHouseholdUI_spawn(id, ), world| {
            BrowserHouseholdUI::spawn(id, world)
        }, false
    );
}