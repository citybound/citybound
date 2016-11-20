#![feature(plugin)]
#![plugin(clippy)]
#![allow(no_effect, unnecessary_operation)]
#![feature(proc_macro)]
extern crate descartes;
#[macro_use]
pub extern crate glium;
extern crate kay;
#[macro_use]
extern crate kay_macros;
extern crate glium_text;

pub use ::descartes::{N, P3, P2, V3, V4, M4, Iso3, Persp3, ToHomogeneous, Norm, Into2d, Into3d, WithUniqueOrthogonal, Inverse, Rotate};
use ::kay::{ID, Recipient, CVec, ActorSystem, Individual, Fate};
use std::collections::HashMap;

use glium::{index, Surface};
pub use glium::backend::glutin_backend::GlutinFacade;

pub struct Renderer {
    pub scenes: HashMap<usize, Scene>,
    pub render_context: RenderContext
}

impl Renderer {
    pub fn new (window: GlutinFacade) -> Renderer {
        Renderer {
            scenes: HashMap::new(),
            render_context: RenderContext::new(window)
        }
    }
}

impl Individual for Renderer {}

#[derive(Copy, Clone)]
pub enum Control {Setup, Render, Submit}

#[derive(Copy, Clone)]
pub struct SetupInScene {pub renderer_id: ID, pub scene_id: usize}

#[derive(Copy, Clone)]
pub struct RenderToScene {pub renderer_id: ID, pub scene_id: usize}

impl Recipient<Control> for Renderer {
    fn receive(&mut self, msg: &Control) -> Fate {match *msg {
        Control::Setup => {
            for (scene_id, scene) in &self.scenes {
                for renderable in &scene.renderables {
                    *renderable << SetupInScene{renderer_id: Self::id(), scene_id: *scene_id};
                }
            }
            Fate::Live
        },

        Control::Render => {
            for (scene_id, mut scene) in &mut self.scenes {
                for batch in (&mut scene).batches.values_mut() {
                    batch.instances.clear();
                }
                for renderable in &scene.renderables {
                    *renderable << RenderToScene{renderer_id: Self::id(), scene_id: *scene_id};
                }
            }
            Fate::Live
        }

        Control::Submit => {
            for scene in self.scenes.values() {
                self.render_context.submit(scene);
            }
            Fate::Live
        }
    }}
}

#[derive(Copy, Clone)]
pub struct MoveEye {pub scene_id: usize, pub delta: V3}

impl Recipient<MoveEye> for Renderer {
    fn receive(&mut self, msg: &MoveEye) -> Fate {match *msg{
        MoveEye{scene_id, delta} => {
            let eye = &mut self.scenes.get_mut(&scene_id).unwrap().eye;
            let eye_direction_2d = (eye.target - eye.position).into_2d().normalize();
            let absolute_delta = delta.x * eye_direction_2d.into_3d()
                + delta.y * eye_direction_2d.orthogonal().into_3d()
                + V3::new(0.0, 0.0, delta.z);
            eye.position += absolute_delta;
            eye.target += absolute_delta;
            Fate::Live
        }
    }}
}

#[derive(Compact, Clone)]
pub struct AddBatch {pub scene_id: usize, pub batch_id: usize, pub thing: Thing}

impl Recipient<AddBatch> for Renderer {
    fn receive(&mut self, msg: &AddBatch) -> Fate {match *msg {
        AddBatch{scene_id, batch_id, ref thing} => {
            self.scenes.get_mut(&scene_id).unwrap().batches.insert(batch_id, Batch::new(thing.clone(), Vec::new()));
            Fate::Live
        }
    }}
}

#[derive(Compact, Clone)]
pub struct UpdateThing {pub scene_id: usize, pub thing_id: usize, pub thing: Thing, pub instance: Instance}

impl Recipient<UpdateThing> for Renderer {
    fn receive(&mut self, msg: &UpdateThing) -> Fate {match *msg {
        UpdateThing{scene_id, thing_id, ref thing, instance} => {
            self.scenes.get_mut(&scene_id).unwrap().things.insert(thing_id, (thing.clone(), instance));
            Fate::Live
        }
    }}
}

#[derive(Copy, Clone)]
pub struct AddInstance {pub scene_id: usize, pub batch_id: usize, pub position: Instance}

impl Recipient<AddInstance> for Renderer {
    fn receive(&mut self, msg: &AddInstance) -> Fate {match *msg {
        AddInstance{scene_id, batch_id, position} => {
            self.scenes.get_mut(&scene_id).unwrap().batches.get_mut(&batch_id).unwrap().instances.push(position);
            Fate::Live
        }
    }}
}

#[derive(Copy, Clone)]
pub struct Project2dTo3d {pub scene_id: usize, pub position_2d: P2, pub requester: ID}

#[derive(Copy, Clone)]
pub struct Projected3d{pub position_3d: P3}

impl Recipient<Project2dTo3d> for Renderer {
    fn receive(&mut self, msg: &Project2dTo3d) -> Fate {match *msg{
        Project2dTo3d{scene_id, position_2d, requester} => {
            let eye = &self.scenes.get_mut(&scene_id).unwrap().eye;
            let frame_size = self.render_context.window.get_framebuffer_dimensions();

            // mouse is on the close plane of the frustum
            let normalized_2d_position = V4::new(
                (position_2d.x / (frame_size.0 as N)) * 2.0 - 1.0,
                (-position_2d.y / (frame_size.1 as N)) * 2.0 + 1.0,
                -1.0,
                1.0
            );

            let inverse_view = Iso3::look_at_rh(
                &eye.position,
                &eye.target,
                &eye.up
            ).to_homogeneous().inverse().unwrap();
            let inverse_perspective = Persp3::new(
                frame_size.0 as f32 / frame_size.1 as f32,
                eye.field_of_view,
                0.1,
                1000.0
            ).to_matrix().inverse().unwrap();

            // converts from frustum to position relative to camera
            let mut position_from_camera = inverse_perspective * normalized_2d_position;
            // reinterpret that as a vector (direction)
            position_from_camera.w = 0.0;
            // convert into world coordinates
            let direction_into_world = inverse_view * position_from_camera;

            let direction_into_world_3d = V3::new(direction_into_world.x, direction_into_world.y, direction_into_world.z);// / direction_into_world.w;

            let distance =  -eye.position.z / direction_into_world_3d.z;
            let position_in_world = eye.position + distance * direction_into_world_3d;

            requester << Projected3d{position_3d: position_in_world};
            Fate::Live
        }
    }}
}

pub fn setup(system: &mut ActorSystem, renderer: Renderer) {
    system.add_individual(renderer);
    system.add_unclearable_inbox::<Control, Renderer>();
    system.add_unclearable_inbox::<AddBatch, Renderer>();
    system.add_unclearable_inbox::<AddInstance, Renderer>();
    system.add_unclearable_inbox::<UpdateThing, Renderer>();
    system.add_unclearable_inbox::<MoveEye, Renderer>();
    system.add_unclearable_inbox::<Project2dTo3d, Renderer>();

    Renderer::id() << Control::Setup;
}

pub struct Scene {
    pub eye: Eye,
    pub batches: HashMap<usize, Batch>,
    pub things: HashMap<usize, (Thing, Instance)>,
    pub renderables: Vec<ID>,
    pub debug_text: String
}

impl Scene {
    pub fn new() -> Scene {
        Scene{
            eye: Eye{
                position: P3::new(-5.0, -5.0, 5.0),
                target: P3::new(0.0, 0.0, 0.0),
                up: V3::new(0.0, 0.0, 1.0),
                field_of_view: 0.3 * ::std::f32::consts::PI
            },
            batches: HashMap::new(),
            things: HashMap::new(),
            renderables: Vec::new(),
            debug_text: String::new()
        }
    }
}

impl Default for Scene {
    fn default() -> Self {Self::new()}
}

pub struct Eye {
    pub position: P3,
    pub target: P3,
    pub up: V3,
    pub field_of_view: f32
}

#[derive(Compact, Debug)]
pub struct Thing {
    pub vertices: CVec<Vertex>,
    pub indices: CVec<u16>
}

impl Thing {
    pub fn new(vertices: Vec<Vertex>, indices: Vec<u16>) -> Thing {
        Thing{vertices: vertices.into(), indices: indices.into()}
    }
}

impl Clone for Thing {
    fn clone(&self) -> Thing {
        Thing {
            vertices: self.vertices.to_vec().into(),
            indices: self.indices.to_vec().into()
        }
    }
}

impl ::std::ops::Add for Thing {
    type Output = Thing;

    fn add(self, rhs: Thing) -> Thing {
        let self_n_vertices = self.vertices.len();
        Thing::new(
            self.vertices.iter().chain(rhs.vertices.iter()).cloned().collect(),
            self.indices.iter().cloned().chain(rhs.indices.iter().map(|i| *i + self_n_vertices as u16)).collect()
        )
    }
}

impl ::std::ops::AddAssign for Thing {
    fn add_assign(&mut self, rhs: Thing) {
        let self_n_vertices = self.vertices.len();
        for vertex in rhs.vertices.iter().cloned() {
            self.vertices.push(vertex);
        }
        for index in rhs.indices.iter() {
            self.indices.push(index + self_n_vertices as u16)
        }
    }
}

impl ::std::iter::Sum for Thing {
    fn sum<I: Iterator<Item=Thing>>(iter: I) -> Thing {
        let mut summed_thing = Thing{vertices: CVec::new(), indices: CVec::new()};
        for thing in iter {
            summed_thing += thing;
        }
        summed_thing
    }
}

impl<'a> ::std::ops::AddAssign<&'a Thing> for Thing {
    fn add_assign(&mut self, rhs: &'a Thing) {
        let self_n_vertices = self.vertices.len();
        for vertex in rhs.vertices.iter().cloned() {
            self.vertices.push(vertex);
        }
        for index in rhs.indices.iter() {
            self.indices.push(index + self_n_vertices as u16)
        }
    }
}

impl<'a> ::std::iter::Sum<&'a Thing> for Thing {
    fn sum<I: Iterator<Item=&'a Thing>>(iter: I) -> Thing {
        let mut summed_thing = Thing{vertices: CVec::new(), indices: CVec::new()};
        for thing in iter {
            summed_thing += thing;
        }
        summed_thing
    }
}

pub struct Batch {
    prototype: Thing,
    pub instances: Vec<Instance>
}

impl Batch {
    pub fn new(prototype: Thing, instances: Vec<Instance>) -> Batch {
        Batch{prototype: prototype, instances: instances}
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Vertex {
    pub position: [f32; 3]
}
implement_vertex!(Vertex, position);

#[derive(Copy, Clone)]
pub struct Instance {
    pub instance_position: [f32; 3],
    pub instance_direction: [f32; 2],
    pub instance_color: [f32; 3]
}
implement_vertex!(Instance, instance_position, instance_direction, instance_color);

impl Instance {
    pub fn with_color(color: [f32; 3]) -> Instance {
        Instance{
            instance_position: [0.0, 0.0, 0.0],
            instance_direction: [1.0, 0.0],
            instance_color: color
        }
    } 
}

pub struct RenderContext {
    pub window: GlutinFacade,
    batch_program: glium::Program,
    text_system: glium_text::TextSystem,
    font: glium_text::FontTexture
}

impl RenderContext {
    #[allow(redundant_closure)]
    pub fn new (window: GlutinFacade) -> RenderContext {
        RenderContext{
            batch_program: program!(&window,
                140 => {
                    vertex: include_str!("shader/solid_140.glslv"),
                    fragment: include_str!("shader/solid_140.glslf")
                }
            ).unwrap(),
            text_system: glium_text::TextSystem::new(&window),
            font: glium_text::FontTexture::new(
                &window,
                ::std::fs::File::open(&::std::path::Path::new("fonts/ClearSans-Regular.ttf")).unwrap(),
                64
            ).unwrap(),
            window: window,
        }
    }

    pub fn submit (&self, scene: &Scene) {
        let mut target = self.window.draw();

        let view : [[f32; 4]; 4] = *Iso3::look_at_rh(
            &scene.eye.position,
            &scene.eye.target,
            &scene.eye.up
        ).to_homogeneous().as_ref();
        let perspective : [[f32; 4]; 4] = *Persp3::new(
            target.get_dimensions().0 as f32 / target.get_dimensions().1 as f32,
            scene.eye.field_of_view,
            0.1,
            1000.0
        ).to_matrix().as_ref();
        
        let uniforms = uniform! {
            view: view,
            perspective: perspective
        };

        let params = glium::DrawParameters {
            depth: glium::Depth {
                test: glium::draw_parameters::DepthTest::IfLess,
                write: true,
                .. Default::default()
            },
            .. Default::default()
        };
        
        // draw a frame
        target.clear_color_and_depth((1.0, 1.0, 1.0, 1.0), 1.0);

        for batch in scene.batches.values() {
            let vertices = glium::VertexBuffer::new(&self.window, &batch.prototype.vertices).unwrap();
            let indices = glium::IndexBuffer::new(&self.window, index::PrimitiveType::TrianglesList, &batch.prototype.indices).unwrap();
            let instances = glium::VertexBuffer::dynamic(&self.window, batch.instances.as_slice()).unwrap();
            target.draw((&vertices, instances.per_instance().unwrap()), &indices, &self.batch_program, &uniforms, &params).unwrap();
        }

        for &(ref thing, instance) in scene.things.values() {
            let vertices = glium::VertexBuffer::new(&self.window, &thing.vertices).unwrap();
            let indices = glium::IndexBuffer::new(&self.window, index::PrimitiveType::TrianglesList, &thing.indices).unwrap();
            let instances = glium::VertexBuffer::new(&self.window, &[instance]).unwrap();
            target.draw((&vertices, instances.per_instance().unwrap()), &indices, &self.batch_program, &uniforms, &params).unwrap();
        }

        let text = glium_text::TextDisplay::new(&self.text_system, &self.font, scene.debug_text.as_str());
        let text_matrix = [
            [0.05, 0.0, 0.0, 0.0],
            [0.0, 0.05, 0.0, 0.0],
            [0.0, 0.0, 0.05, 0.0],
            [-0.9, 0.8, 0.0, 1.0f32]
        ];

        glium_text::draw(&text, &self.text_system, &mut target, text_matrix, (0.0, 0.0, 0.0, 1.0));

        target.finish().unwrap();
    }
}