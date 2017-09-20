pub use kay::External;
pub use descartes::{N, P3, P2, V3, V4, M4, Iso3, Persp3, ToHomogeneous, Norm, Into2d, Into3d,
                    WithUniqueOrthogonal, Inverse, Rotate};

use glium::{self, index};
use glium::backend::glutin_backend::GlutinFacade;

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

use compact::CDict;
use kay::{ActorSystem, Fate, World};
use kay::swarm::Swarm;
use itertools::Itertools;

pub trait GroupIndividual {
    fn render_to_group(&mut self, group: GroupID, individual_id: u16, world: &mut World);
}

pub struct GroupInner {
    instance_color: [f32; 3],
    individual_id: u16,
    is_decal: bool,
    living_individuals: HashMap<GroupIndividualID, Geometry>,
    frozen_individuals: HashMap<GroupIndividualID, Geometry>,
    n_frozen_groups: usize,
    n_total_groups: usize,
    cached_frozen_individuals_clean_in: HashMap<RendererID, ()>,
}

#[derive(Compact, Clone)]
pub struct Group {
    id: GroupID,
    inner: External<GroupInner>,
}

impl ::std::ops::Deref for Group {
    type Target = GroupInner;

    fn deref(&self) -> &GroupInner {
        &self.inner
    }
}

impl ::std::ops::DerefMut for Group {
    fn deref_mut(&mut self) -> &mut GroupInner {
        &mut self.inner
    }
}

impl Group {
    pub fn spawn(
        id: GroupID,
        instance_color: &[f32; 3],
        individual_id: u16,
        is_decal: bool,
        _: &mut World,
    ) -> Group {
        Group {
            id,
            inner: External::new(GroupInner {
                instance_color: *instance_color,
                individual_id: individual_id,
                is_decal: is_decal,
                living_individuals: HashMap::new(),
                frozen_individuals: HashMap::new(),
                n_frozen_groups: 0,
                cached_frozen_individuals_clean_in: HashMap::new(),
                n_total_groups: 0,
            }),
        }
    }

    pub fn initial_add(&mut self, id: GroupIndividualID, world: &mut World) {
        id.render_to_group(self.id, self.individual_id, world);
    }

    pub fn update(&mut self, id: GroupIndividualID, geometry: &Geometry, _: &mut World) {
        if self.frozen_individuals.get(&id).is_none() {
            self.living_individuals.insert(id, geometry.clone());
        }
    }

    pub fn freeze(&mut self, id: GroupIndividualID, _: &mut World) {
        if let Some(geometry) = self.living_individuals.remove(&id) {
            self.frozen_individuals.insert(id, geometry);
            self.cached_frozen_individuals_clean_in = HashMap::new();
        }
    }

    pub fn unfreeze(&mut self, id: GroupIndividualID, _: &mut World) {
        if let Some(geometry) = self.frozen_individuals.remove(&id) {
            self.living_individuals.insert(id, geometry);
            self.cached_frozen_individuals_clean_in = HashMap::new();
        }
    }

    pub fn remove(&mut self, id: GroupIndividualID, _: &mut World) {
        self.living_individuals.remove(&id);
        if self.frozen_individuals.remove(&id).is_some() {
            self.cached_frozen_individuals_clean_in = HashMap::new();
        };
    }
}

use {Renderable, RendererID, RenderableID, MSG_Renderable_setup_in_scene,
     MSG_Renderable_render_to_scene};

impl Renderable for Group {
    fn setup_in_scene(&mut self, _renderer_id: RendererID, _scene_id: usize, _: &mut World) {}

    fn render_to_scene(&mut self, renderer_id: RendererID, scene_id: usize, world: &mut World) {
        // TODO: this introduces 1 frame delay
        for id in self.living_individuals.keys() {
            id.render_to_group(self.id, self.individual_id, world);
        }

        let clean_in_renderer = self.cached_frozen_individuals_clean_in
            .get(&renderer_id)
            .is_none();

        if clean_in_renderer {
            let new_n_frozen_groups = {
                let cached_frozen_individuals_grouped = self.frozen_individuals
                    .values()
                    .cloned()
                    .coalesce(|a, b| if a.vertices.len() + b.vertices.len() >
                        u16::max_value() as usize
                    {
                        Err((a, b))
                    } else {
                        Ok(a + b)
                    });


                let mut new_n_frozen_groups = 0;

                for frozen_group in cached_frozen_individuals_grouped {
                    renderer_id.update_individual(
                        scene_id,
                        self.individual_id + self.n_frozen_groups as u16,
                        frozen_group,
                        Instance {
                            instance_position: [0.0, 0.0, -0.1],
                            instance_direction: [1.0, 0.0],
                            instance_color: self.instance_color,
                        },
                        self.is_decal,
                        world,
                    );

                    new_n_frozen_groups += 1;
                }

                new_n_frozen_groups
            };

            self.n_frozen_groups = new_n_frozen_groups;

            self.cached_frozen_individuals_clean_in.insert(
                renderer_id,
                (),
            );
        }

        let mut new_n_total_groups = self.n_frozen_groups;

        {
            let living_individual_groups = self.living_individuals.values().cloned().coalesce(
                |a, b| if a.vertices.len() + b.vertices.len() >
                    u16::max_value() as
                        usize
                {
                    Err((a, b))
                } else {
                    Ok(a + b)
                },
            );

            for living_individual_group in living_individual_groups {
                renderer_id.update_individual(
                    scene_id,
                    self.individual_id + new_n_total_groups as u16,
                    living_individual_group,
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
                for individual_to_empty_id in new_n_total_groups..self.n_total_groups {
                    renderer_id.update_individual(
                        scene_id,
                        self.individual_id + individual_to_empty_id as u16,
                        Geometry::new(vec![], vec![]),
                        Instance::with_color([0.0, 0.0, 0.0]),
                        self.is_decal,
                        world,
                    );
                }
            }
        }

        self.n_total_groups = new_n_total_groups;
    }
}

pub struct Batch {
    pub vertices: glium::VertexBuffer<Vertex>,
    pub indices: glium::IndexBuffer<u16>,
    pub instances: Vec<Instance>,
    pub clear_every_frame: bool,
    pub is_decal: bool,
}

impl Batch {
    pub fn new(prototype: Geometry, window: &GlutinFacade) -> Batch {
        Batch {
            vertices: glium::VertexBuffer::new(window, &prototype.vertices).unwrap(),
            indices: glium::IndexBuffer::new(
                window,
                index::PrimitiveType::TrianglesList,
                &prototype.indices,
            ).unwrap(),
            instances: Vec::new(),
            clear_every_frame: true,
            is_decal: false,
        }
    }

    pub fn new_individual(
        geometry: Geometry,
        instance: Instance,
        is_decal: bool,
        window: &GlutinFacade,
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
            is_decal: is_decal,
        }
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.add(Swarm::<Group>::new(), |_| {});

    auto_setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;