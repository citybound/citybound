use monet::{Thing, Instance};
use compact::CDict;
use kay::{ActorSystem, Fate, World};
use kay::swarm::Swarm;
use itertools::Itertools;

#[derive(Copy, Clone)]
enum ThingLocation {
    Living(usize),
    Frozen(usize),
}

pub trait RenderableToCollector {
    fn render_to_collector(
        &mut self,
        collector: ThingCollectorID,
        base_thing_id: u16,
        world: &mut World,
    );
}

#[derive(Compact, Clone)]
pub struct ThingCollector {
    id: ThingCollectorID,
    instance_color: [f32; 3],
    base_thing_id: u16,
    is_decal: bool,
    living_things: CDict<RenderableToCollectorID, Thing>,
    frozen_things: CDict<RenderableToCollectorID, Thing>,
    n_frozen_groups: usize,
    n_total_groups: usize,
    cached_frozen_things_dirty: bool,
}

impl ThingCollector {
    pub fn spawn(
        id: ThingCollectorID,
        instance_color: &[f32; 3],
        base_thing_id: u16,
        is_decal: bool,
        _: &mut World,
    ) -> ThingCollector {
        ThingCollector {
            id,
            instance_color: *instance_color,
            base_thing_id: base_thing_id,
            is_decal: is_decal,
            living_things: CDict::new(),
            frozen_things: CDict::new(),
            n_frozen_groups: 0,
            cached_frozen_things_dirty: false,
            n_total_groups: 0,
        }
    }

    pub fn initial_add(&mut self, id: RenderableToCollectorID, world: &mut World) {
        id.render_to_collector(self.id, self.base_thing_id, world);
    }

    pub fn update(&mut self, id: RenderableToCollectorID, thing: &Thing, _: &mut World) {
        if self.frozen_things.get(id).is_none() {
            self.living_things.insert(id, thing.clone());
        }
    }

    pub fn freeze(&mut self, id: RenderableToCollectorID, _: &mut World) {
        if let Some(thing) = self.living_things.remove(id) {
            self.frozen_things.insert(id, thing);
            self.cached_frozen_things_dirty = true;
        }
    }

    pub fn unfreeze(&mut self, id: RenderableToCollectorID, _: &mut World) {
        if let Some(thing) = self.frozen_things.remove(id) {
            self.living_things.insert(id, thing);
            self.cached_frozen_things_dirty = true;
        }
    }

    pub fn remove(&mut self, id: RenderableToCollectorID, _: &mut World) {
        self.living_things.remove(id);
        if self.frozen_things.remove(id).is_some() {
            self.cached_frozen_things_dirty = true;
        };
    }
}

use monet::{RendererID, Renderable, RenderableID, MSG_Renderable_setup_in_scene,
            MSG_Renderable_render_to_scene};

impl Renderable for ThingCollector {
    fn setup_in_scene(&mut self, _renderer_id: RendererID, _scene_id: usize, _: &mut World) {}

    fn render_to_scene(&mut self, renderer_id: RendererID, scene_id: usize, world: &mut World) {
        // TODO: this introduces 1 frame delay
        for id in self.living_things.keys() {
            id.render_to_collector(self.id, self.base_thing_id, world);
        }

        if self.cached_frozen_things_dirty {
            let cached_frozen_things_grouped = self.frozen_things.values().cloned().coalesce(
                |a, b| if a.vertices.len() + b.vertices.len() >
                    u16::max_value() as
                        usize
                {
                    Err((a, b))
                } else {
                    Ok(a + b)
                },
            );
            self.cached_frozen_things_dirty = false;

            self.n_frozen_groups = 0;

            for frozen_group in cached_frozen_things_grouped {
                renderer_id.update_thing(
                    scene_id,
                    self.base_thing_id + self.n_frozen_groups as u16,
                    frozen_group,
                    Instance {
                        instance_position: [0.0, 0.0, -0.1],
                        instance_direction: [1.0, 0.0],
                        instance_color: self.instance_color,
                    },
                    self.is_decal,
                    world,
                );

                self.n_frozen_groups += 1;
            }
        }

        let living_thing_groups = self.living_things.values().cloned().coalesce(
            |a, b| if a.vertices.len() + b.vertices.len() >
                u16::max_value() as
                    usize
            {
                Err((a, b))
            } else {
                Ok(a + b)
            },
        );

        let mut new_n_total_groups = self.n_frozen_groups;

        for living_thing_group in living_thing_groups {
            renderer_id.update_thing(
                scene_id,
                self.base_thing_id + new_n_total_groups as u16,
                living_thing_group,
                Instance {
                    instance_position: [0.0, 0.0, -0.1],
                    instance_direction: [1.0, 0.0],
                    instance_color: self.instance_color,
                },
                self.is_decal,
                world,
            );

            new_n_total_groups += 1;
        }

        if new_n_total_groups > self.n_total_groups {
            for thing_to_empty_id in new_n_total_groups..self.n_total_groups {
                renderer_id.update_thing(
                    scene_id,
                    self.base_thing_id + thing_to_empty_id as u16,
                    Thing::new(vec![], vec![]),
                    Instance::with_color([0.0, 0.0, 0.0]),
                    self.is_decal,
                    world,
                );
            }
        }

        self.n_total_groups = new_n_total_groups;
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.add(Swarm::<ThingCollector>::new(), |_| {});

    auto_setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;