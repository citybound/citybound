//! This is all auto-generated. Do not touch.
#![cfg_attr(rustfmt, rustfmt_skip)]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)] #[serde(transparent)]
pub struct LandUseUIID {
    _raw_id: RawID
}

pub struct LandUseUIRepresentative;

impl ActorOrActorTrait for LandUseUIRepresentative {
    type ID = LandUseUIID;
}

impl TypedID for LandUseUIID {
    type Target = LandUseUIRepresentative;

    fn from_raw(id: RawID) -> Self {
        LandUseUIID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl<A: Actor + LandUseUI> TraitIDFrom<A> for LandUseUIID {}

impl LandUseUIID {
    pub fn on_building_constructed(&self, id: BuildingID, lot: Lot, households: CVec < HouseholdID >, style: BuildingStyle, world: &mut World) {
        world.send(self.as_raw(), MSG_LandUseUI_on_building_constructed(id, lot, households, style));
    }
    
    pub fn on_building_destructed(&self, id: BuildingID, world: &mut World) {
        world.send(self.as_raw(), MSG_LandUseUI_on_building_destructed(id));
    }
    
    pub fn on_building_ui_info(&self, id: BuildingID, style: BuildingStyle, households: CVec < HouseholdID >, world: &mut World) {
        world.send(self.as_raw(), MSG_LandUseUI_on_building_ui_info(id, style, households));
    }

    pub fn register_trait(system: &mut ActorSystem) {
        system.register_trait::<LandUseUIRepresentative>();
        system.register_trait_message::<MSG_LandUseUI_on_building_constructed>();
        system.register_trait_message::<MSG_LandUseUI_on_building_destructed>();
        system.register_trait_message::<MSG_LandUseUI_on_building_ui_info>();
    }

    pub fn register_implementor<A: Actor + LandUseUI>(system: &mut ActorSystem) {
        system.register_implementor::<A, LandUseUIRepresentative>();
        system.add_handler::<A, _, _>(
            |&MSG_LandUseUI_on_building_constructed(id, ref lot, ref households, style), instance, world| {
                instance.on_building_constructed(id, lot, households, style, world); Fate::Live
            }, false
        );
        
        system.add_handler::<A, _, _>(
            |&MSG_LandUseUI_on_building_destructed(id), instance, world| {
                instance.on_building_destructed(id, world); Fate::Live
            }, false
        );
        
        system.add_handler::<A, _, _>(
            |&MSG_LandUseUI_on_building_ui_info(id, style, ref households), instance, world| {
                instance.on_building_ui_info(id, style, households, world); Fate::Live
            }, false
        );
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_LandUseUI_on_building_constructed(pub BuildingID, pub Lot, pub CVec < HouseholdID >, pub BuildingStyle);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_LandUseUI_on_building_destructed(pub BuildingID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_LandUseUI_on_building_ui_info(pub BuildingID, pub BuildingStyle, pub CVec < HouseholdID >);



#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    LandUseUIID::register_trait(system);
    
}