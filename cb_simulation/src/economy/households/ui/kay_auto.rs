//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;

#[derive(Serialize, Deserialize)] #[serde(transparent)]
pub struct HouseholdUIID {
    _raw_id: RawID
}

impl Copy for HouseholdUIID {}
impl Clone for HouseholdUIID { fn clone(&self) -> Self { *self } }
impl ::std::fmt::Debug for HouseholdUIID {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "HouseholdUIID({:?})", self._raw_id)
    }
}
impl ::std::hash::Hash for HouseholdUIID {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self._raw_id.hash(state);
    }
}
impl PartialEq for HouseholdUIID {
    fn eq(&self, other: &HouseholdUIID) -> bool {
        self._raw_id == other._raw_id
    }
}
impl Eq for HouseholdUIID {}

pub struct HouseholdUIRepresentative;

impl ActorOrActorTrait for HouseholdUIRepresentative {
    type ID = HouseholdUIID;
}

impl TypedID for HouseholdUIID {
    type Target = HouseholdUIRepresentative;

    fn from_raw(id: RawID) -> Self {
        HouseholdUIID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl<Act: Actor + HouseholdUI> TraitIDFrom<Act> for HouseholdUIID {}

impl HouseholdUIID {
    pub fn on_household_ui_info(self, id: HouseholdID, core: HouseholdCore, world: &mut World) {
        world.send(self.as_raw(), MSG_HouseholdUI_on_household_ui_info(id, core));
    }

    pub fn register_trait(system: &mut ActorSystem) {
        system.register_trait::<HouseholdUIRepresentative>();
        system.register_trait_message::<MSG_HouseholdUI_on_household_ui_info>();
    }

    pub fn register_implementor<Act: Actor + HouseholdUI>(system: &mut ActorSystem) {
        system.register_implementor::<Act, HouseholdUIRepresentative>();
        system.add_handler::<Act, _, _>(
            |&MSG_HouseholdUI_on_household_ui_info(id, ref core), instance, world| {
                instance.on_household_ui_info(id, core, world); Fate::Live
            }, false
        );
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_HouseholdUI_on_household_ui_info(pub HouseholdID, pub HouseholdCore);



#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    HouseholdUIID::register_trait(system);
    
}