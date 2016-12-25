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
extern crate rusttype;
extern crate fnv;
extern crate unicode_normalization;

pub use ::descartes::{N, P3, P2, V3, V4, M4, Iso3, Persp3, ToHomogeneous, Norm, Into2d, Into3d, WithUniqueOrthogonal, Inverse, Rotate};
use ::kay::{ID, Recipient, CVec, ActorSystem, Individual, Fate};
use fnv::FnvHashMap;
use std::borrow::Cow;

use glium::{index, Surface};
pub use glium::backend::glutin_backend::GlutinFacade;

pub struct Renderer {
    pub scenes: Vec<Scene>,
    pub render_context: RenderContext
}

impl Renderer {
    pub fn new (window: GlutinFacade) -> Renderer {
        Renderer {
            scenes: Vec::new(),
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
            for (scene_id, scene) in self.scenes.iter().enumerate() {
                for renderable in &scene.renderables {
                    *renderable << SetupInScene{renderer_id: Self::id(), scene_id: scene_id};
                }
            }
            Fate::Live
        },

        Control::Render => {
            for (scene_id, mut scene) in self.scenes.iter_mut().enumerate() {
                for batch_to_clear in (&mut scene).batches.values_mut().filter(|batch| batch.clear_every_frame) {
                    batch_to_clear.instances.clear();
                }
                for renderable in &scene.renderables {
                    *renderable << RenderToScene{renderer_id: Self::id(), scene_id: scene_id};
                }
            }
            Fate::Live
        }

        Control::Submit => {
            for scene in &mut self.scenes {
                self.render_context.submit(scene);
                scene.debug_text.clear();
            }
            Fate::Live
        }
    }}
}

#[derive(Copy, Clone)]
pub enum Movement{
    Shift(V3),
    Zoom(N),
    Rotate(N)
}

#[derive(Copy, Clone)]
pub struct MoveEye {pub scene_id: usize, pub movement: Movement}
#[derive(Copy, Clone)]
pub struct EyeMoved {pub eye: Eye, pub movement: Movement}

impl Recipient<MoveEye> for Renderer {
    fn receive(&mut self, msg: &MoveEye) -> Fate {match *msg{
        MoveEye{scene_id, movement} => {
            match movement {
                Movement::Shift(delta) => {
                    let eye = &mut self.scenes[scene_id].eye;
                    let eye_direction_2d = (eye.target - eye.position).into_2d().normalize();
                    let absolute_delta = delta.x * eye_direction_2d.into_3d()
                        + delta.y * eye_direction_2d.orthogonal().into_3d()
                        + V3::new(0.0, 0.0, delta.z);
                    eye.position += absolute_delta * (eye.position.z / 100.0);
                    eye.target += absolute_delta * (eye.position.z / 100.0);
                },
                Movement::Zoom(delta) => {
                    let eye = &mut self.scenes[scene_id].eye;
                    let eye_direction = (eye.target - eye.position).normalize();
                    if (eye.target - eye.position).norm() > 30.0 || delta < 0.0 {
                        eye.position += eye_direction * delta * (eye.position.z / 100.0);
                    }
                },
                Movement::Rotate(delta) => {
                    let eye = &mut self.scenes[scene_id].eye;
                    let relative_eye_position = eye.position - eye.target;
                    let iso = Iso3::new(V3::new(0.0, 0.0, 0.0), V3::new(0.0, 0.0, delta));
                    let rotated_relative_eye_position = iso.rotate(&relative_eye_position);
                    eye.position = eye.target + rotated_relative_eye_position;
                }
            }
            for &id in &self.scenes[scene_id].eye_listeners {
                id << EyeMoved{eye: self.scenes[scene_id].eye, movement: movement};
            }
            Fate::Live
        },
    }}
}

#[derive(Copy, Clone)]
pub struct AddEyeListener{pub scene_id: usize, pub listener: ID}

impl Recipient<AddEyeListener> for Renderer {
    fn receive(&mut self, msg: &AddEyeListener) -> Fate {match *msg {
        AddEyeListener{scene_id, listener} => {
            self.scenes[scene_id].eye_listeners.push(listener);
            Fate::Live
        }
    }}
}

#[derive(Compact, Clone)]
pub struct AddBatch {pub scene_id: usize, pub batch_id: u16, pub thing: Thing}

impl Recipient<AddBatch> for Renderer {
    fn receive(&mut self, msg: &AddBatch) -> Fate {match *msg {
        AddBatch{scene_id, batch_id, ref thing} => {
            let window = &self.render_context.window;
            self.scenes[scene_id].batches.insert(batch_id, Batch::new(thing.clone(), window));
            Fate::Live
        }
    }}
}

#[derive(Compact, Clone)]
pub struct UpdateThing {pub scene_id: usize, pub thing_id: u16, pub thing: Thing, pub instance: Instance, pub is_decal: bool}

impl Recipient<UpdateThing> for Renderer {
    fn receive(&mut self, msg: &UpdateThing) -> Fate {match *msg {
        UpdateThing{scene_id, thing_id, ref thing, instance, is_decal} => {
            self.scenes[scene_id].batches.insert(thing_id, Batch::new_thing(thing.clone(), instance, is_decal, &self.render_context.window));
            Fate::Live
        }
    }}
}

#[derive(Copy, Clone)]
pub struct AddInstance {pub scene_id: usize, pub batch_id: u16, pub instance: Instance}

impl Recipient<AddInstance> for Renderer {
    fn receive(&mut self, msg: &AddInstance) -> Fate {match *msg {
        AddInstance{scene_id, batch_id, instance} => {
            self.scenes[scene_id].batches.get_mut(&batch_id).unwrap().instances.push(instance);
            Fate::Live
        }
    }}
}

#[derive(Compact, Clone)]
pub struct AddSeveralInstances {pub scene_id: usize, pub batch_id: u16, pub instances: CVec<Instance>}

impl Recipient<AddSeveralInstances> for Renderer {
    fn receive(&mut self, msg: &AddSeveralInstances) -> Fate {match *msg {
        AddSeveralInstances{scene_id, batch_id, ref instances} => {
            self.scenes[scene_id].batches.get_mut(&batch_id).unwrap().instances.extend_from_slice(instances);
            Fate::Live
        }
    }}
}

#[derive(Compact, Clone)]
pub struct AddDebugText {pub scene_id: usize, pub key: CVec<char>, pub text: CVec<char>, pub color: [f32; 4]}

impl Recipient<AddDebugText> for Renderer {
    fn receive(&mut self, msg: &AddDebugText) -> Fate {match *msg {
        AddDebugText{scene_id, ref key, ref text, ref color} => {
            self.scenes[scene_id].debug_text.insert(
                key.iter().cloned().collect(),
                (text.iter().cloned().collect(), *color)
            );
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
            let eye = &self.scenes[scene_id].eye;
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
    system.add_unclearable_inbox::<AddSeveralInstances, Renderer>();
    system.add_unclearable_inbox::<MoveEye, Renderer>();
    system.add_unclearable_inbox::<AddEyeListener, Renderer>();
    system.add_unclearable_inbox::<AddDebugText, Renderer>();
    system.add_unclearable_inbox::<UpdateThing, Renderer>();
    system.add_unclearable_inbox::<Project2dTo3d, Renderer>();

    Renderer::id() << Control::Setup;
}

pub struct Scene {
    pub eye: Eye,
    pub eye_listeners: CVec<ID>,
    pub batches: FnvHashMap<u16, Batch>,
    pub renderables: Vec<ID>,
    pub debug_text: std::collections::BTreeMap<String, (String, [f32; 4])>
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
            eye_listeners: CVec::new(),
            batches: FnvHashMap::default(),
            renderables: Vec::new(),
            debug_text: std::collections::BTreeMap::new()
        }
    }
}

impl Default for Scene {
    fn default() -> Self {Self::new()}
}

#[derive(Copy, Clone)]
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
    vertices: glium::VertexBuffer<Vertex>,
    indices: glium::IndexBuffer<u16>,
    instances: Vec<Instance>,
    clear_every_frame: bool,
    is_decal: bool
}

impl Batch {
    pub fn new(prototype: Thing, window: &GlutinFacade) -> Batch {
        Batch{
            vertices: glium::VertexBuffer::new(window, &prototype.vertices).unwrap(),
            indices: glium::IndexBuffer::new(window, index::PrimitiveType::TrianglesList, &prototype.indices).unwrap(),
            instances: Vec::new(),
            clear_every_frame: true,
            is_decal: false
        }
    }

    pub fn new_thing(thing: Thing, instance: Instance, is_decal: bool, window: &GlutinFacade) -> Batch {
        Batch{
            vertices: glium::VertexBuffer::new(window, &thing.vertices).unwrap(),
            indices: glium::IndexBuffer::new(window, index::PrimitiveType::TrianglesList, &thing.indices).unwrap(),
            instances: vec![instance],
            clear_every_frame: false,
            is_decal: is_decal
        }
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
    text_program: glium::Program,
    text_cache_tex: glium::Texture2d,
    text_font: rusttype::Font<'static>,
    text_cache: rusttype::gpu_cache::Cache
}

impl RenderContext {
    #[allow(redundant_closure)]
    pub fn new (window: GlutinFacade) -> RenderContext {
        let dpi_factor = window.get_window().unwrap().hidpi_factor();
        let (text_cache_width, text_cache_height) = (512 * dpi_factor as u32, 512 * dpi_factor as u32);

        RenderContext{
            batch_program: program!(&window, 140 => {
                vertex: include_str!("shader/solid_140.glslv"),
                fragment: include_str!("shader/solid_140.glslf")
            }).unwrap(),
            text_program: program!(&window, 140 => {
                vertex: include_str!("shader/text_140.glslv"),
                fragment: include_str!("shader/text_140.glslf")
            }).unwrap(),
            text_cache_tex: glium::Texture2d::with_format(
                &window,
                glium::texture::RawImage2d{
                    data: Cow::Owned(vec![128u8; text_cache_width as usize * text_cache_height as usize]),
                    width: text_cache_width, height: text_cache_height,
                    format: glium::texture::ClientFormat::U8
                },
                glium::texture::UncompressedFloatFormat::U8,
                glium::texture::MipmapsOption::NoMipmap
            ).unwrap(),
            text_font: rusttype::FontCollection::from_bytes(
                include_bytes!("../../../fonts/ClearSans-Regular.ttf") as &[u8]
            ).into_font().unwrap(),
            text_cache: rusttype::gpu_cache::Cache::new(text_cache_width, text_cache_height, 0.1, 0.1),
            window: window,
        }
    }

    pub fn submit (&mut self, scene: &Scene) {
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
            50000.0
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

        let decal_params = glium::DrawParameters {
            depth: glium::Depth {
                test: glium::draw_parameters::DepthTest::Overwrite,
                write: false,
                .. Default::default()
            },
            .. Default::default()
        };

        // draw a frame
        target.clear_color_and_depth((1.0, 1.0, 1.0, 1.0), 1.0);

        let mut render_debug_text = String::from("Renderer:\n");

        let mut batches_todo = scene.batches.iter().collect::<Vec<_>>();
        batches_todo.sort_by_key(|&(batch_id, _)| batch_id);

        for (i, &Batch{ref vertices, ref indices, ref instances, is_decal, ..}) in batches_todo {
            if instances.len() > 1 {
                render_debug_text.push_str(format!("batch{}: {} instances\n", i, instances.len()).as_str());
            }
            let instance_buffer = glium::VertexBuffer::new(&self.window, instances).unwrap();
            target.draw(
                (vertices, instance_buffer.per_instance().unwrap()),
                indices,
                &self.batch_program,
                &uniforms,
                if is_decal {&decal_params} else {&params}
            ).unwrap();
        }

        let (width, dpi_factor) = {
            let window = self.window.get_window().unwrap();
            (window.get_inner_size_pixels().unwrap().0, window.hidpi_factor())
        };
        let glyphs = layout_paragraph(
            &self.text_font,
            rusttype::Scale::uniform(14.0 * dpi_factor),
            width,
            (scene.debug_text.iter().map(|(key, &(ref text, _))|
                format!("{}:\n{}\n", key, text)
            ).collect::<String>() + render_debug_text.as_str()).as_str(),
            scene.debug_text.iter().map(|(key, &(ref text, ref color))|
                (key.len() + text.len() + 3, *color)
            ).chain(Some((render_debug_text.len(), [0.0, 0.0, 0.0, 0.5]))).collect()
        );

        for &(ref glyph, _) in &glyphs {
            self.text_cache.queue_glyph(0, glyph.clone());
        }
        {
            let text_cache_tex = &mut self.text_cache_tex;
            self.text_cache.cache_queued(|rect, data| {
                text_cache_tex.main_level().write(glium::Rect {
                    left: rect.min.x,
                    bottom: rect.min.y,
                    width: rect.width(),
                    height: rect.height()
                }, glium::texture::RawImage2d {
                    data: Cow::Borrowed(data),
                    width: rect.width(),
                    height: rect.height(),
                    format: glium::texture::ClientFormat::U8
                });
            }).unwrap();
        }

        let text_uniforms = uniform! {
            tex: self.text_cache_tex.sampled().magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest)
        };

        let text_vertex_buffer = {
            #[derive(Copy, Clone)]
            struct TextVertex {
                position: [f32; 2],
                tex_coords: [f32; 2],
                color: [f32; 4]
            }

            implement_vertex!(TextVertex, position, tex_coords, color);
            let (screen_width, screen_height) = {
                let (w, h) = self.window.get_framebuffer_dimensions();
                (w as f32, h as f32)
            };
            let origin = rusttype::point(0.0, 0.0);
            let vertices: Vec<TextVertex> = glyphs.iter().flat_map(|&(ref g, color)| {
                if let Ok(Some((uv_rect, screen_rect))) = self.text_cache.rect_for(0, g) {
                    let gl_rect = rusttype::Rect {
                        min: origin
                            + (rusttype::vector(screen_rect.min.x as f32 / screen_width - 0.5,
                                      1.0 - screen_rect.min.y as f32 / screen_height - 0.5)) * 2.0,
                        max: origin
                            + (rusttype::vector(screen_rect.max.x as f32 / screen_width - 0.5,
                                      1.0 - screen_rect.max.y as f32 / screen_height - 0.5)) * 2.0
                    };
                    vec![
                        TextVertex {
                            position: [gl_rect.min.x, gl_rect.max.y],
                            tex_coords: [uv_rect.min.x, uv_rect.max.y],
                            color: color
                        },
                        TextVertex {
                            position: [gl_rect.min.x,  gl_rect.min.y],
                            tex_coords: [uv_rect.min.x, uv_rect.min.y],
                            color: color
                        },
                        TextVertex {
                            position: [gl_rect.max.x,  gl_rect.min.y],
                            tex_coords: [uv_rect.max.x, uv_rect.min.y],
                            color: color
                        },
                        TextVertex {
                            position: [gl_rect.max.x,  gl_rect.min.y],
                            tex_coords: [uv_rect.max.x, uv_rect.min.y],
                            color: color },
                        TextVertex {
                            position: [gl_rect.max.x, gl_rect.max.y],
                            tex_coords: [uv_rect.max.x, uv_rect.max.y],
                            color: color
                        },
                        TextVertex {
                            position: [gl_rect.min.x, gl_rect.max.y],
                            tex_coords: [uv_rect.min.x, uv_rect.max.y],
                            color: color
                        }]
                } else {
                    vec![]
                }
            }).collect();

            glium::VertexBuffer::new(
                &self.window,
                &vertices).unwrap()
        };

        target.draw(&text_vertex_buffer,
                    glium::index::NoIndices(glium::index::PrimitiveType::TrianglesList),
                    &self.text_program, &text_uniforms,
                    &glium::DrawParameters {
                        blend: glium::Blend::alpha_blending(),
                        ..Default::default()
                    }).unwrap();

        target.finish().unwrap();
    }
}

fn layout_paragraph<'a>(font: &'a rusttype::Font,
                        scale: rusttype::Scale,
                        width: u32,
                        text: &str,
                        mut colors: Vec<(usize, [f32; 4])>) -> Vec<(rusttype::PositionedGlyph<'a>, [f32; 4])> {
    use unicode_normalization::UnicodeNormalization;
    let mut result = Vec::new();
    let v_metrics = font.v_metrics(scale);
    let advance_height = v_metrics.ascent - v_metrics.descent + v_metrics.line_gap;
    let mut caret = rusttype::point(0.0, v_metrics.ascent);
    let mut last_glyph_id = None;
    for c in text.nfc() {
        let color = colors[0].1;
        colors[0].0 -= 1;
        if colors[0].0 == 0 && colors.len() > 1 {
            colors.remove(0);
        }
        if c.is_control() {
            if c == '\n' {
                caret = rusttype::point(0.0, caret.y + advance_height);
            }
            continue;
        }
        let base_glyph = if let Some(glyph) = font.glyph(c) {
            glyph
        } else {
            continue;
        };
        if let Some(id) = last_glyph_id.take() {
            caret.x += font.pair_kerning(scale, id, base_glyph.id());
        }
        last_glyph_id = Some(base_glyph.id());
        let mut glyph = base_glyph.scaled(scale).positioned(caret);
        if let Some(bb) = glyph.pixel_bounding_box() {
            if bb.max.x > width as i32 {
                caret = rusttype::point(0.0, caret.y + advance_height);
                glyph = glyph.into_unpositioned().positioned(caret);
                last_glyph_id = None;
            }
        }
        caret.x += glyph.unpositioned().h_metrics().advance_width;
        result.push((glyph, color));
    }
    result
}
