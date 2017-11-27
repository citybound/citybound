pub use kay::{External, TypedID};
pub use descartes::{N, P3, P2, V3, V4, M4, Iso3, Persp3, ToHomogeneous, Norm, Into2d, Into3d,
                    WithUniqueOrthogonal, Inverse, Rotate};

use glium::{self, index};
use glium::backend::glutin::Display;

use compact::CVec;
use std::collections::HashMap;

#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    pub position: [f32; 3],
}

implement_vertex!(Vertex, position);

#[derive(Copy, Clone)]
pub struct Instance {
    pub instance_position: [f32; 3],
    pub instance_direction: [f32; 2],
    pub instance_color: [f32; 3],
}

implement_vertex!(
    Instance,
    instance_position,
    instance_direction,
    instance_color
);

impl Instance {
    pub fn with_color(color: [f32; 3]) -> Instance {
        Instance {
            instance_position: [0.0, 0.0, 0.0],
            instance_direction: [1.0, 0.0],
            instance_color: color,
        }
    }
}

#[derive(Compact, Debug)]
pub struct Geometry {
    pub vertices: CVec<Vertex>,
    pub indices: CVec<u16>,
}

impl Geometry {
    pub fn new(vertices: Vec<Vertex>, indices: Vec<u16>) -> Geometry {
        Geometry {
            vertices: vertices.into(),
            indices: indices.into(),
        }
    }

    pub fn empty() -> Geometry {
        Geometry {
            vertices: CVec::new(),
            indices: CVec::new(),
        }
    }
}

impl Clone for Geometry {
    fn clone(&self) -> Geometry {
        Geometry {
            vertices: self.vertices.to_vec().into(),
            indices: self.indices.to_vec().into(),
        }
    }
}

impl ::std::ops::Add for Geometry {
    type Output = Geometry;

    fn add(mut self, rhs: Geometry) -> Geometry {
        let self_n_vertices = self.vertices.len();
        self.vertices.extend_from_copy_slice(&rhs.vertices);
        self.indices.extend(rhs.indices.iter().map(|i| {
            *i + self_n_vertices as u16
        }));
        self
    }
}

impl ::std::ops::AddAssign for Geometry {
    fn add_assign(&mut self, rhs: Geometry) {
        let self_n_vertices = self.vertices.len();
        for vertex in rhs.vertices.iter().cloned() {
            self.vertices.push(vertex);
        }
        for index in rhs.indices.iter() {
            self.indices.push(index + self_n_vertices as u16)
        }
    }
}

impl ::std::iter::Sum for Geometry {
    fn sum<I: Iterator<Item = Geometry>>(iter: I) -> Geometry {
        let mut summed_geometry = Geometry {
            vertices: CVec::new(),
            indices: CVec::new(),
        };
        for geometry in iter {
            summed_geometry += geometry;
        }
        summed_geometry
    }
}

impl<'a> ::std::ops::AddAssign<&'a Geometry> for Geometry {
    fn add_assign(&mut self, rhs: &'a Geometry) {
        let self_n_vertices = self.vertices.len();
        for vertex in rhs.vertices.iter().cloned() {
            self.vertices.push(vertex);
        }
        for index in rhs.indices.iter() {
            self.indices.push(index + self_n_vertices as u16)
        }
    }
}

impl<'a> ::std::iter::Sum<&'a Geometry> for Geometry {
    fn sum<I: Iterator<Item = &'a Geometry>>(iter: I) -> Geometry {
        let mut summed_geometry = Geometry {
            vertices: CVec::new(),
            indices: CVec::new(),
        };
        for geometry in iter {
            summed_geometry += geometry;
        }
        summed_geometry
    }
}

use kay::{ActorSystem, World};
use itertools::Itertools;

pub trait GrouperIndividual {
    fn render_to_grouper(&mut self, grouper: GrouperID, base_individual_id: u16, world: &mut World);
}

#[derive(Clone)]
struct GrouperRendererState {
    n_living_groups: usize,
    n_frozen_groups: usize,
    frozen_up_to_date: bool,
}

pub struct GrouperInner {
    instance_color: [f32; 3],
    base_individual_id: u16,
    is_decal: bool,
    living_individuals: HashMap<GrouperIndividualID, Geometry>,
    frozen_individuals: HashMap<GrouperIndividualID, Geometry>,
    living_groups: Vec<Geometry>,
    frozen_groups: Vec<Geometry>,
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
        base_individual_id: u16,
        is_decal: bool,
        _: &mut World,
    ) -> Grouper {
        Grouper {
            id,
            inner: External::new(GrouperInner {
                instance_color: *instance_color,
                base_individual_id: base_individual_id,
                is_decal: is_decal,
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

    pub fn update(&mut self, id: GrouperIndividualID, geometry: &Geometry, _: &mut World) {
        if self.frozen_individuals.get(&id).is_none() {
            self.living_individuals.insert(id, geometry.clone());
        }
    }

    pub fn add_frozen(&mut self, id: GrouperIndividualID, geometry: &Geometry, _: &mut World) {
        self.frozen_individuals.insert(id, geometry.clone());
        self.frozen_up_to_date = false;
        for state in self.renderer_state.values_mut() {
            state.frozen_up_to_date = false;
        }
    }

    pub fn freeze(&mut self, id: GrouperIndividualID, _: &mut World) {
        if let Some(geometry) = self.living_individuals.remove(&id) {
            self.frozen_individuals.insert(id, geometry);
            self.frozen_up_to_date = false;
            for state in self.renderer_state.values_mut() {
                state.frozen_up_to_date = false;
            }
        }
    }

    pub fn unfreeze(&mut self, id: GrouperIndividualID, _: &mut World) {
        if let Some(geometry) = self.frozen_individuals.remove(&id) {
            self.living_individuals.insert(id, geometry);
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
}

use {Renderable, RendererID, RenderableID, MSG_Renderable_setup_in_scene,
     MSG_Renderable_render_to_scene};

impl Renderable for Grouper {
    fn setup_in_scene(&mut self, _renderer_id: RendererID, _scene_id: usize, _: &mut World) {}

    fn render_to_scene(
        &mut self,
        renderer_id: RendererID,
        scene_id: usize,
        _frame: usize,
        world: &mut World,
    ) {

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
            if (i as u16) < FROZEN_OFFSET {
                renderer_id.update_individual(
                    scene_id,
                    self.base_individual_id + i as u16,
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
                scene_id,
                self.base_individual_id + i as u16,
                Geometry::empty(),
                Instance::with_color([0.0, 0.0, 0.0]),
                self.is_decal,
                world,
            );
        }

        new_renderer_state.n_living_groups = self.living_groups.len();

        const FROZEN_OFFSET: u16 = 100;

        if !new_renderer_state.frozen_up_to_date {
            for (i, frozen_group) in self.frozen_groups.iter().enumerate() {
                renderer_id.update_individual(
                    scene_id,
                    self.base_individual_id + FROZEN_OFFSET + i as u16,
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
                    scene_id,
                    self.base_individual_id + FROZEN_OFFSET + i as u16,
                    Geometry::empty(),
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

pub struct Batch {
    pub vertices: glium::VertexBuffer<Vertex>,
    pub indices: glium::IndexBuffer<u16>,
    pub instances: Vec<Instance>,
    pub clear_every_frame: bool,
    pub full_frame_instance_end: Option<usize>,
    pub is_decal: bool,
    pub frame: usize,
}

impl Batch {
    pub fn new(prototype: Geometry, window: &Display) -> Batch {
        Batch {
            vertices: glium::VertexBuffer::new(window, &prototype.vertices).unwrap(),
            indices: glium::IndexBuffer::new(
                window,
                index::PrimitiveType::TrianglesList,
                &prototype.indices,
            ).unwrap(),
            instances: Vec::new(),
            full_frame_instance_end: None,
            clear_every_frame: true,
            is_decal: false,
            frame: 0,
        }
    }

    pub fn new_individual(
        geometry: Geometry,
        instance: Instance,
        is_decal: bool,
        window: &Display,
    ) -> Batch {
        Batch {
            vertices: glium::VertexBuffer::new(window, &geometry.vertices).unwrap(),
            indices: glium::IndexBuffer::new(
                window,
                index::PrimitiveType::TrianglesList,
                &geometry.indices,
            ).unwrap(),
            instances: vec![instance],
            clear_every_frame: false,
            full_frame_instance_end: None,
            is_decal: is_decal,
            frame: 0,
        }
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.register::<Grouper>();

    auto_setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;
