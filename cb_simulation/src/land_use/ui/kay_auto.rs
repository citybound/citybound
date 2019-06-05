//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;

#[derive(Serialize, Deserialize)] #[serde(transparent)]
pub struct LandUseUIID {
    _raw_id: RawID
}

impl Copy for LandUseUIID {}
impl Clone for LandUseUIID { fn clone(&self) -> Self { *self } }
impl ::std::fmt::Debug for LandUseUIID {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "LandUseUIID({:?})", self._raw_id)
    }
}
impl ::std::hash::Hash for LandUseUIID {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self._raw_id.hash(state);
    }
}
impl PartialEq for LandUseUIID {
    fn eq(&self, other: &LandUseUIID) -> bool {
        self._raw_id == other._raw_id
    }
}
impl Eq for LandUseUIID {}

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

impl<Act: Actor + LandUseUI> TraitIDFrom<Act> for LandUseUIID {}

impl LandUseUIID {
    pub fn on_building_constructed(self, id: BuildingID, lot: Lot, households: CVec < HouseholdID >, style: BuildingStyle, world: &mut World) {
        world.send(self.as_raw(), MSG_LandUseUI_on_building_constructed(id, lot, households, style));
    }
    
    pub fn on_building_destructed(self, id: BuildingID, world: &mut World) {
        world.send(self.as_raw(), MSG_LandUseUI_on_building_destructed(id));
    }
    
    pub fn on_building_ui_info(self, id: BuildingID, style: BuildingStyle, households: CVec < HouseholdID >, world: &mut World) {
        world.send(self.as_raw(), MSG_LandUseUI_on_building_ui_info(id, style, households));
    }

    pub fn register_trait(system: &mut ActorSystem) {
        system.register_trait::<LandUseUIRepresentative>();
        system.register_trait_message::<MSG_LandUseUI_on_building_constructed>();
        system.register_trait_message::<MSG_LandUseUI_on_building_destructed>();
        system.register_trait_message::<MSG_LandUseUI_on_building_ui_info>();
    }

    pub fn register_implementor<Act: Actor + LandUseUI>(system: &mut ActorSystem) {
        system.register_implementor::<Act, LandUseUIRepresentative>();
        system.add_handler::<Act, _, _>(
            |&MSG_LandUseUI_on_building_constructed(id, ref lot, ref households, style), instance, world| {
                instance.on_building_constructed(id, lot, households, style, world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_LandUseUI_on_building_destructed(id), instance, world| {
                instance.on_building_destructed(id, world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
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