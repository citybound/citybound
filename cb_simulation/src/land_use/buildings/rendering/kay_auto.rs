//! This is all auto-generated. Do not touch.
#![cfg_attr(rustfmt, rustfmt_skip)]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;





impl BuildingID {
    pub fn get_render_info(&self, ui: LandUseUIID, world: &mut World) {
        world.send(self.as_raw(), MSG_Building_get_render_info(ui));
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Building_get_render_info(pub LandUseUIID);


#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    
    
    system.add_handler::<Building, _, _>(
        |&MSG_Building_get_render_info(ui), instance, world| {
            instance.get_render_info(ui, world); Fate::Live
        }, false
    );
}