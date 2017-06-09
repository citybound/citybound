// ALL OF THIS IS AUTO-GENERATED, DON'T TOUCH
use kay::ActorSystem;
use super::*;
#[derive(Copy, Clone)]
pub struct RendererID(ID);

impl RendererID {
    pub fn in_world(world: &mut World) -> RendererID {
        RendererID(world.id::<Renderer>())
    }

    pub fn add_eye_listener(&self, scene_id: usize, listener: ID, world: &mut World) {
        world.send(self.0, _KAY_MSG_add_eye_listener(scene_id, listener));
    }
    pub fn add_batch(&self, scene_id: usize, batch_id: u16, thing: Thing, world: &mut World) {
        world.send(self.0, _KAY_MSG_add_batch(scene_id, batch_id, thing));
    }
    pub fn update_thing(&self, scene_id: usize, thing_id: u16, thing: Thing, instance: Instance, is_decal: bool, world: &mut World) {
        world.send(self.0, _KAY_MSG_update_thing(scene_id, thing_id, thing, instance, is_decal));
    }
    pub fn add_instance(&self, scene_id: usize, batch_id: u16, instance: Instance, world: &mut World) {
        world.send(self.0, _KAY_MSG_add_instance(scene_id, batch_id, instance));
    }
    pub fn add_several_instances(&self, scene_id: usize, batch_id: u16, instances: CVec<Instance>, world: &mut World) {
        world.send(self.0, _KAY_MSG_add_several_instances(scene_id, batch_id, instances));
    }
}

#[allow(non_camel_case_types)]
#[derive(Compact, Clone)]
struct _KAY_MSG_add_eye_listener(usize, ID);
#[allow(non_camel_case_types)]
#[derive(Compact, Clone)]
struct _KAY_MSG_add_batch(usize, u16, Thing);
#[allow(non_camel_case_types)]
#[derive(Compact, Clone)]
struct _KAY_MSG_update_thing(usize, u16, Thing, Instance, bool);
#[allow(non_camel_case_types)]
#[derive(Compact, Clone)]
struct _KAY_MSG_add_instance(usize, u16, Instance);
#[allow(non_camel_case_types)]
#[derive(Compact, Clone)]
struct _KAY_MSG_add_several_instances(usize, u16, CVec<Instance>);

pub fn auto_setup(system: &mut ActorSystem, initial: Renderer) {
    system.add(initial, |mut definer| {
        definer.on_critical(|&_KAY_MSG_add_eye_listener(scene_id, listener), actor, world| {
            actor.add_eye_listener(scene_id, listener, world);
            Fate::Live
        });

        definer.on_critical(|&_KAY_MSG_add_batch(scene_id, batch_id, ref thing), actor, world| {
            actor.add_batch(scene_id, batch_id, thing, world);
            Fate::Live
        });

        definer.on_critical(|&_KAY_MSG_update_thing(scene_id, thing_id, ref thing, ref instance, is_decal), actor, world| {
            actor.update_thing(scene_id, thing_id, thing, instance, is_decal, world);
            Fate::Live
        });

        definer.on_critical(|&_KAY_MSG_add_instance(scene_id, batch_id, instance), actor, world| {
            actor.add_instance(scene_id, batch_id, instance, world);
            Fate::Live
        });

        definer.on_critical(|&_KAY_MSG_add_several_instances(scene_id, batch_id, ref instances), actor, world| {
            actor.add_several_instances(scene_id, batch_id, instances, world);
            Fate::Live
        });
    });
}