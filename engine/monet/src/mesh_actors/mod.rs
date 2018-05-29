pub use kay::{External, TypedID};
pub use super::mesh::{Mesh, Batch, Instance};

use kay::{ActorSystem, World};
use std::collections::HashMap;
use itertools::Itertools;

pub trait GrouperIndividual {
    fn render_to_grouper(&mut self, grouper: GrouperID, base_individual_id: u32, world: &mut World);
}

#[derive(Clone)]
struct GrouperRendererState {
    n_living_groups: usize,
    n_frozen_groups: usize,
    frozen_up_to_date: bool,
}

pub struct GrouperInner {
    instance_color: [f32; 3],
    base_individual_id: u32,
    is_decal: bool,
    living_individuals: HashMap<GrouperIndividualID, Mesh>,
    frozen_individuals: HashMap<GrouperIndividualID, Mesh>,
    living_groups: Vec<Mesh>,
    frozen_groups: Vec<Mesh>,
    frozen_up_to_date: bool,
    renderer_state: HashMap<RendererID, GrouperRendererState>,
}

#[derive(Compact, Clone)]
pub struct Grouper {
    id: GrouperID,
    inner: External<GrouperInner>,
}

impl ::std::ops::Deref for Grouper {
    type Target = GrouperInner;

    fn deref(&self) -> &GrouperInner {
        &self.inner
    }
}

impl ::std::ops::DerefMut for Grouper {
    fn deref_mut(&mut self) -> &mut GrouperInner {
        &mut self.inner
    }
}

impl Grouper {
    pub fn spawn(
        id: GrouperID,
        instance_color: &[f32; 3],
        base_individual_id: u32,
        is_decal: bool,
        _: &mut World,
    ) -> Grouper {
        Grouper {
            id,
            inner: External::new(GrouperInner {
                instance_color: *instance_color,
                base_individual_id,
                is_decal,
                living_individuals: HashMap::new(),
                frozen_individuals: HashMap::new(),
                living_groups: Vec::new(),
                frozen_groups: Vec::new(),
                frozen_up_to_date: true,
                renderer_state: HashMap::new(),
            }),
        }
    }

    pub fn initial_add(&mut self, id: GrouperIndividualID, world: &mut World) {
        id.render_to_grouper(self.id, self.base_individual_id, world);
    }

    pub fn update(&mut self, id: GrouperIndividualID, mesh: &Mesh, _: &mut World) {
        if self.frozen_individuals.get(&id).is_none() {
            self.living_individuals.insert(id, mesh.clone());
        }
    }

    pub fn add_frozen(&mut self, id: GrouperIndividualID, mesh: &Mesh, _: &mut World) {
        self.frozen_individuals.insert(id, mesh.clone());
        self.frozen_up_to_date = false;
        for state in self.renderer_state.values_mut() {
            state.frozen_up_to_date = false;
        }
    }

    pub fn freeze(&mut self, id: GrouperIndividualID, _: &mut World) {
        if let Some(mesh) = self.living_individuals.remove(&id) {
            self.frozen_individuals.insert(id, mesh);
            self.frozen_up_to_date = false;
            for state in self.renderer_state.values_mut() {
                state.frozen_up_to_date = false;
            }
        }
    }

    pub fn unfreeze(&mut self, id: GrouperIndividualID, _: &mut World) {
        if let Some(mesh) = self.frozen_individuals.remove(&id) {
            self.living_individuals.insert(id, mesh);
            self.frozen_up_to_date = false;
            for state in self.renderer_state.values_mut() {
                state.frozen_up_to_date = false;
            }
        }
    }

    pub fn remove(&mut self, id: GrouperIndividualID, _: &mut World) {
        self.living_individuals.remove(&id);
        if self.frozen_individuals.remove(&id).is_some() {
            self.frozen_up_to_date = false;
            for state in self.renderer_state.values_mut() {
                state.frozen_up_to_date = false;
            }
        };
    }

    pub fn clear(&mut self, _: &mut World) {
        self.living_individuals.clear();
        self.frozen_individuals.clear();
        self.frozen_up_to_date = false;
        for state in self.renderer_state.values_mut() {
            state.frozen_up_to_date = false;
        }
    }
}

use {Renderable, RendererID, RenderableID};

impl Renderable for Grouper {
    fn setup_in_scene(&mut self, _renderer_id: RendererID, _: &mut World) {}

    fn render_to_scene(&mut self, renderer_id: RendererID, _frame: usize, world: &mut World) {

        // kinda ugly way to enforce only one update per "global" frame
        if renderer_id.as_raw().machine == self.id.as_raw().machine {
            // TODO: this introduces 1 frame delay
            for id in self.living_individuals.keys() {
                id.render_to_grouper(self.id, self.base_individual_id, world);
            }

            self.living_groups = self.living_individuals
                .values()
                .cloned()
                .coalesce(|a, b| if a.vertices.len() + b.vertices.len() >
                    u16::max_value() as usize
                {
                    Err((a, b))
                } else {
                    Ok(a + b)
                })
                .collect();
        }

        if !self.frozen_up_to_date {
            self.frozen_groups = self.frozen_individuals
                .values()
                .cloned()
                .coalesce(|a, b| if a.vertices.len() + b.vertices.len() >
                    u16::max_value() as usize
                {
                    Err((a, b))
                } else {
                    Ok(a + b)
                })
                .collect();

            self.frozen_up_to_date = true;
        }

        let mut new_renderer_state = self.renderer_state.get(&renderer_id).cloned().unwrap_or(
            GrouperRendererState {
                n_living_groups: 0,
                n_frozen_groups: 0,
                frozen_up_to_date: false,
            },
        );

        for (i, living_group) in self.living_groups.iter().enumerate() {
            if (i as u32) < FROZEN_OFFSET {
                renderer_id.update_individual(
                    self.base_individual_id + i as u32,
                    living_group.clone(),
                    Instance {
                        instance_position: [0.0, 0.0, -0.1],
                        instance_direction: [1.0, 0.0],
                        instance_color: self.instance_color,
                    },
                    self.is_decal,
                    world,
                );
            }
        }

        for i in self.living_groups.len()..
            ::std::cmp::min(new_renderer_state.n_living_groups, FROZEN_OFFSET as usize)
        {
            renderer_id.update_individual(
                self.base_individual_id + i as u32,
                Mesh::empty(),
                Instance::with_color([0.0, 0.0, 0.0]),
                self.is_decal,
                world,
            );
        }

        new_renderer_state.n_living_groups = self.living_groups.len();

        const FROZEN_OFFSET: u32 = 100;

        if !new_renderer_state.frozen_up_to_date {
            for (i, frozen_group) in self.frozen_groups.iter().enumerate() {
                renderer_id.update_individual(
                    self.base_individual_id + FROZEN_OFFSET + i as u32,
                    frozen_group.clone(),
                    Instance {
                        instance_position: [0.0, 0.0, -0.1],
                        instance_direction: [1.0, 0.0],
                        instance_color: self.instance_color,
                    },
                    self.is_decal,
                    world,
                );
            }

            for i in self.frozen_groups.len()..new_renderer_state.n_frozen_groups {
                renderer_id.update_individual(
                    self.base_individual_id + FROZEN_OFFSET + i as u32,
                    Mesh::empty(),
                    Instance::with_color([0.0, 0.0, 0.0]),
                    self.is_decal,
                    world,
                );
            }

            new_renderer_state.n_frozen_groups = self.frozen_groups.len();
            new_renderer_state.frozen_up_to_date = true;
        }

        self.renderer_state.insert(renderer_id, new_renderer_state);
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.register::<Grouper>();

    auto_setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;
