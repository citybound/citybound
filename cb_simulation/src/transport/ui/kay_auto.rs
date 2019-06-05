//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;

#[derive(Serialize, Deserialize)] #[serde(transparent)]
pub struct TransportUIID {
    _raw_id: RawID
}

impl Copy for TransportUIID {}
impl Clone for TransportUIID { fn clone(&self) -> Self { *self } }
impl ::std::fmt::Debug for TransportUIID {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "TransportUIID({:?})", self._raw_id)
    }
}
impl ::std::hash::Hash for TransportUIID {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self._raw_id.hash(state);
    }
}
impl PartialEq for TransportUIID {
    fn eq(&self, other: &TransportUIID) -> bool {
        self._raw_id == other._raw_id
    }
}
impl Eq for TransportUIID {}

pub struct TransportUIRepresentative;

impl ActorOrActorTrait for TransportUIRepresentative {
    type ID = TransportUIID;
}

impl TypedID for TransportUIID {
    type Target = TransportUIRepresentative;

    fn from_raw(id: RawID) -> Self {
        TransportUIID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl<Act: Actor + TransportUI> TraitIDFrom<Act> for TransportUIID {}

impl TransportUIID {
    pub fn on_lane_constructed(self, id: RawID, lane_path: LinePath, is_switch: bool, on_intersection: bool, world: &mut World) {
        world.send(self.as_raw(), MSG_TransportUI_on_lane_constructed(id, lane_path, is_switch, on_intersection));
    }
    
    pub fn on_lane_destructed(self, id: RawID, is_switch: bool, on_intersection: bool, world: &mut World) {
        world.send(self.as_raw(), MSG_TransportUI_on_lane_destructed(id, is_switch, on_intersection));
    }
    
    pub fn on_car_info(self, from_lane: RawID, infos: CVec < CarRenderInfo >, world: &mut World) {
        world.send(self.as_raw(), MSG_TransportUI_on_car_info(from_lane, infos));
    }

    pub fn register_trait(system: &mut ActorSystem) {
        system.register_trait::<TransportUIRepresentative>();
        system.register_trait_message::<MSG_TransportUI_on_lane_constructed>();
        system.register_trait_message::<MSG_TransportUI_on_lane_destructed>();
        system.register_trait_message::<MSG_TransportUI_on_car_info>();
    }

    pub fn register_implementor<Act: Actor + TransportUI>(system: &mut ActorSystem) {
        system.register_implementor::<Act, TransportUIRepresentative>();
        system.add_handler::<Act, _, _>(
            |&MSG_TransportUI_on_lane_constructed(id, ref lane_path, is_switch, on_intersection), instance, world| {
                instance.on_lane_constructed(id, lane_path, is_switch, on_intersection, world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_TransportUI_on_lane_destructed(id, is_switch, on_intersection), instance, world| {
                instance.on_lane_destructed(id, is_switch, on_intersection, world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_TransportUI_on_car_info(from_lane, ref infos), instance, world| {
                instance.on_car_info(from_lane, infos, world); Fate::Live
            }, false
        );
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_TransportUI_on_lane_constructed(pub RawID, pub LinePath, pub bool, pub bool);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_TransportUI_on_lane_destructed(pub RawID, pub bool, pub bool);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_TransportUI_on_car_info(pub RawID, pub CVec < CarRenderInfo >);



impl LaneID {
    pub fn get_car_info(self, ui: TransportUIID, world: &mut World) {
        world.send(self.as_raw(), MSG_Lane_get_car_info(ui));
    }
    
    pub fn get_render_info(self, ui: TransportUIID, world: &mut World) {
        world.send(self.as_raw(), MSG_Lane_get_render_info(ui));
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Lane_get_car_info(pub TransportUIID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Lane_get_render_info(pub TransportUIID);




impl SwitchLaneID {
    pub fn get_render_info(self, ui: TransportUIID, world: &mut World) {
        world.send(self.as_raw(), MSG_SwitchLane_get_render_info(ui));
    }
    
    pub fn get_car_info(self, ui: TransportUIID, world: &mut World) {
        world.send(self.as_raw(), MSG_SwitchLane_get_car_info(ui));
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_SwitchLane_get_render_info(pub TransportUIID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_SwitchLane_get_car_info(pub TransportUIID);


#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    TransportUIID::register_trait(system);
    
    system.add_handler::<Lane, _, _>(
        |&MSG_Lane_get_car_info(ui), instance, world| {
            instance.get_car_info(ui, world); Fate::Live
        }, false
    );
    
    system.add_handler::<Lane, _, _>(
        |&MSG_Lane_get_render_info(ui), instance, world| {
            instance.get_render_info(ui, world); Fate::Live
        }, false
    );
    
    system.add_handler::<SwitchLane, _, _>(
        |&MSG_SwitchLane_get_render_info(ui), instance, world| {
            instance.get_render_info(ui, world); Fate::Live
        }, false
    );
    
    system.add_handler::<SwitchLane, _, _>(
        |&MSG_SwitchLane_get_car_info(ui), instance, world| {
            instance.get_car_info(ui, world); Fate::Live
        }, false
    );
}