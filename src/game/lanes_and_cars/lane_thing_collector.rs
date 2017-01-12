use monet::{Thing, Instance};
use compact::CDict;
use kay::{ID, Individual, Recipient, ActorSystem, Fate};
use ::std::marker::PhantomData;
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

impl<T: 'static + Clone> Individual for ThingCollector<T> {}

use ::monet::SetupInScene;

impl<T: Clone> Recipient<SetupInScene> for ThingCollector<T> {
    fn receive(&mut self, _msg: &SetupInScene) -> Fate {
        Fate::Live
    }
}

#[derive(Compact, Clone)]
pub enum Control {
    Update(ID, Thing),
    Freeze(ID),
    Unfreeze(ID),
    Remove(ID),
}

impl<T: Clone> Recipient<Control> for ThingCollector<T> {
    fn receive(&mut self, msg: &Control) -> Fate {
        match *msg {
            Control::Update(id, ref thing) => {
                match self.frozen_things.get(id) {
                    Some(_) => Fate::Live,
                    None => {
                        self.living_things.insert(id, thing.clone());
                        Fate::Live
                    }
                }
            }
            Control::Freeze(id) => {
                if let Some(thing) = self.living_things.remove(id) {
                    self.frozen_things.insert(id, thing);
                    self.cached_frozen_things_dirty = true;
                }
                Fate::Live
            }
            Control::Unfreeze(id) => {
                if let Some(thing) = self.frozen_things.remove(id) {
                    self.living_things.insert(id, thing);
                    self.cached_frozen_things_dirty = true;
                }
                Fate::Live
            }
            Control::Remove(id) => {
                self.living_things.remove(id);
                if self.frozen_things.remove(id).is_some() {
                    self.cached_frozen_things_dirty = true;
                };
                Fate::Live
            }
        }
    }
}

use ::monet::RenderToScene;
use ::monet::UpdateThing;

#[derive(Copy, Clone)]
pub struct RenderToCollector(pub ID);

impl<T: Clone + 'static> Recipient<RenderToScene> for ThingCollector<T> {
    fn receive(&mut self, msg: &RenderToScene) -> Fate {
        match *msg {
            RenderToScene { renderer_id, scene_id } => {
                // TODO: this introduces 1 frame delay
                for id in self.living_things.keys() {
                    *id << RenderToCollector(Self::id());
                }

                if self.cached_frozen_things_dirty {
                    let cached_frozen_things_grouped = self.frozen_things
                        .values()
                        .cloned()
                        .coalesce(|a, b| if a.vertices.len() + b.vertices.len() >
                                            u16::max_value() as usize {
                            Err((a, b))
                        } else {
                            Ok(a + b)
                        });
                    self.cached_frozen_things_dirty = false;

                    self.n_frozen_groups = 0;

                    for frozen_group in cached_frozen_things_grouped {
                        renderer_id <<
                        UpdateThing {
                            scene_id: scene_id,
                            thing_id: self.base_thing_id + self.n_frozen_groups as u16,
                            thing: frozen_group,
                            instance: Instance {
                                instance_position: [0.0, 0.0, -0.1],
                                instance_direction: [1.0, 0.0],
                                instance_color: self.instance_color,
                            },
                            is_decal: self.is_decal,
                        };

                        self.n_frozen_groups += 1;
                    }
                }

                let living_thing_groups = self.living_things
                    .values()
                    .cloned()
                    .coalesce(|a, b| if a.vertices.len() + b.vertices.len() >
                                        u16::max_value() as usize {
                        Err((a, b))
                    } else {
                        Ok(a + b)
                    });

                let mut new_n_total_groups = self.n_frozen_groups;

                for living_thing_group in living_thing_groups {
                    renderer_id <<
                    UpdateThing {
                        scene_id: scene_id,
                        thing_id: self.base_thing_id + new_n_total_groups as u16,
                        thing: living_thing_group,
                        instance: Instance {
                            instance_position: [0.0, 0.0, -0.1],
                            instance_direction: [1.0, 0.0],
                            instance_color: self.instance_color,
                        },
                        is_decal: self.is_decal,
                    };

                    new_n_total_groups += 1;
                }

                if new_n_total_groups > self.n_total_groups {
                    for thing_to_empty_id in new_n_total_groups..self.n_total_groups {
                        renderer_id <<
                        UpdateThing {
                            scene_id: scene_id,
                            thing_id: self.base_thing_id + thing_to_empty_id as u16,
                            thing: Thing::new(vec![], vec![]),
                            instance: Instance::with_color([0.0, 0.0, 0.0]),
                            is_decal: self.is_decal,
                        }
                    }
                }

                self.n_total_groups = new_n_total_groups;

                Fate::Live
            }
        }
    }
}

pub fn setup<T: Clone + 'static>(system: &mut ActorSystem,
                                 instance_color: [f32; 3],
                                 base_thing_id: u16,
                                 is_decal: bool) {
    system.add_individual(ThingCollector::<T> {
        instance_color: instance_color,
        base_thing_id: base_thing_id,
        is_decal: is_decal,
        living_things: CDict::new(),
        frozen_things: CDict::new(),
        n_frozen_groups: 0,
        cached_frozen_things_dirty: false,
        n_total_groups: 0,
        _marker: PhantomData,
    });
    system.add_inbox::<Control, ThingCollector<T>>();
    system.add_inbox::<SetupInScene, ThingCollector<T>>();
    system.add_inbox::<RenderToScene, ThingCollector<T>>();
}
