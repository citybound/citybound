//! This is all auto-generated. Do not touch.
#![cfg_attr(rustfmt, rustfmt_skip)]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)] #[serde(transparent)]
pub struct VegetationUIID {
    _raw_id: RawID
}

pub struct VegetationUIRepresentative;

impl ActorOrActorTrait for VegetationUIRepresentative {
    type ID = VegetationUIID;
}

impl TypedID for VegetationUIID {
    type Target = VegetationUIRepresentative;

    fn from_raw(id: RawID) -> Self {
        VegetationUIID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl<A: Actor + VegetationUI> TraitIDFrom<A> for VegetationUIID {}

impl VegetationUIID {
    pub fn on_plant_spawned(&self, id: PlantID, proto: PlantPrototype, world: &mut World) {
        world.send(self.as_raw(), MSG_VegetationUI_on_plant_spawned(id, proto));
    }
    
    pub fn on_plant_destroyed(&self, id: PlantID, world: &mut World) {
        world.send(self.as_raw(), MSG_VegetationUI_on_plant_destroyed(id));
    }

    pub fn register_trait(system: &mut ActorSystem) {
        system.register_trait::<VegetationUIRepresentative>();
        system.register_trait_message::<MSG_VegetationUI_on_plant_spawned>();
        system.register_trait_message::<MSG_VegetationUI_on_plant_destroyed>();
    }

    pub fn register_implementor<A: Actor + VegetationUI>(system: &mut ActorSystem) {
        system.register_implementor::<A, VegetationUIRepresentative>();
        system.add_handler::<A, _, _>(
            |&MSG_VegetationUI_on_plant_spawned(id, ref proto), instance, world| {
                instance.on_plant_spawned(id, proto, world); Fate::Live
            }, false
        );
        
        system.add_handler::<A, _, _>(
            |&MSG_VegetationUI_on_plant_destroyed(id), instance, world| {
                instance.on_plant_destroyed(id, world); Fate::Live
            }, false
        );
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_VegetationUI_on_plant_spawned(pub PlantID, pub PlantPrototype);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_VegetationUI_on_plant_destroyed(pub PlantID);



impl PlantID {
    pub fn get_render_info(&self, requester: VegetationUIID, world: &mut World) {
        world.send(self.as_raw(), MSG_Plant_get_render_info(requester));
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Plant_get_render_info(pub VegetationUIID);


#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    VegetationUIID::register_trait(system);
    
    system.add_handler::<Plant, _, _>(
        |&MSG_Plant_get_render_info(requester), instance, world| {
            instance.get_render_info(requester, world); Fate::Live
        }, false
    );
}