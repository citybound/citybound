use monet::{Thing, Instance};
use compact::CDict;
use kay::{ID, ActorSystem, Fate};
use std::marker::PhantomData;
use itertools::Itertools;

#[derive(Copy, Clone)]
enum ThingLocation {
    Living(usize),
    Frozen(usize),
}

// TODO: the Clone bound on T is stupid
#[derive(Compact, Clone)]
pub struct ThingCollector<T: Clone> {
    instance_color: [f32; 3],
    base_thing_id: u16,
    is_decal: bool,
    living_things: CDict<ID, Thing>,
    frozen_things: CDict<ID, Thing>,
    n_frozen_groups: usize,
    n_total_groups: usize,
    cached_frozen_things_dirty: bool,
    _marker: PhantomData<*const T>,
}


use monet::MSG_Renderable_setup_in_scene;
use monet::MSG_Renderable_render_to_scene;

pub fn setup<T: Clone + 'static>(
    system: &mut ActorSystem,
    instance_color: [f32; 3],
    base_thing_id: u16,
    is_decal: bool,
) {
    let initial = ThingCollector::<T> {
        instance_color: instance_color,
        base_thing_id: base_thing_id,
        is_decal: is_decal,
        living_things: CDict::new(),
        frozen_things: CDict::new(),
        n_frozen_groups: 0,
        cached_frozen_things_dirty: false,
        n_total_groups: 0,
        _marker: PhantomData,
    };

    system.add(initial, |mut the_collector| {
        let collector_id = the_collector.world().id::<ThingCollector<T>>();

        the_collector.on(|_: &MSG_Renderable_setup_in_scene, _, _| Fate::Live);

        the_collector.on(|control, coll, _| {
            match *control {
                Control::Update(id, ref thing) => {
                    if coll.frozen_things.get(id).is_none() {
                        coll.living_things.insert(id, thing.clone());
                    }
                }
                Control::Freeze(id) => {
                    if let Some(thing) = coll.living_things.remove(id) {
                        coll.frozen_things.insert(id, thing);
                        coll.cached_frozen_things_dirty = true;
                    }
                }
                Control::Unfreeze(id) => {
                    if let Some(thing) = coll.frozen_things.remove(id) {
                        coll.living_things.insert(id, thing);
                        coll.cached_frozen_things_dirty = true;
                    }
                }
                Control::Remove(id) => {
                    coll.living_things.remove(id);
                    if coll.frozen_things.remove(id).is_some() {
                        coll.cached_frozen_things_dirty = true;
                    };
                }
            };
            Fate::Live
        });

        the_collector.on(move |&MSG_Renderable_render_to_scene(renderer_id,
                                              scene_id),
              coll,
              world| {
            // TODO: this introduces 1 frame delay
            for id in coll.living_things.keys() {
                world.send(*id, RenderToCollector(collector_id));
            }

            if coll.cached_frozen_things_dirty {
                let cached_frozen_things_grouped = coll.frozen_things.values().cloned().coalesce(
                    |a, b| {
                        if a.vertices.len() + b.vertices.len() > u16::max_value() as usize {
                            Err((a, b))
                        } else {
                            Ok(a + b)
                        }
                    },
                );
                coll.cached_frozen_things_dirty = false;

                coll.n_frozen_groups = 0;

                for frozen_group in cached_frozen_things_grouped {
                    renderer_id.update_thing(
                        scene_id,
                        coll.base_thing_id + coll.n_frozen_groups as u16,
                        frozen_group,
                        Instance {
                            instance_position: [0.0, 0.0, -0.1],
                            instance_direction: [1.0, 0.0],
                            instance_color: coll.instance_color,
                        },
                        coll.is_decal,
                        world,
                    );

                    coll.n_frozen_groups += 1;
                }
            }

            let living_thing_groups = coll.living_things.values().cloned().coalesce(
                |a, b| if a.vertices.len() + b.vertices.len() >
                    u16::max_value() as
                        usize
                {
                    Err((a, b))
                } else {
                    Ok(a + b)
                },
            );

            let mut new_n_total_groups = coll.n_frozen_groups;

            for living_thing_group in living_thing_groups {
                renderer_id.update_thing(
                    scene_id,
                    coll.base_thing_id + new_n_total_groups as u16,
                    living_thing_group,
                    Instance {
                        instance_position: [0.0, 0.0, -0.1],
                        instance_direction: [1.0, 0.0],
                        instance_color: coll.instance_color,
                    },
                    coll.is_decal,
                    world,
                );

                new_n_total_groups += 1;
            }

            if new_n_total_groups > coll.n_total_groups {
                for thing_to_empty_id in new_n_total_groups..coll.n_total_groups {
                    renderer_id.update_thing(
                        scene_id,
                        coll.base_thing_id + thing_to_empty_id as u16,
                        Thing::new(vec![], vec![]),
                        Instance::with_color([0.0, 0.0, 0.0]),
                        coll.is_decal,
                        world,
                    );
                }
            }

            coll.n_total_groups = new_n_total_groups;

            Fate::Live
        })
    });
}

#[derive(Compact, Clone)]
pub enum Control {
    Update(ID, Thing),
    Freeze(ID),
    Unfreeze(ID),
    Remove(ID),
}

#[derive(Copy, Clone)]
pub struct RenderToCollector(pub ID);
