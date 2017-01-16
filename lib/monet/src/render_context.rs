
use glium;

pub use descartes::{N, P3, P2, V3, V4, M4, Iso3, Persp3, ToHomogeneous, Norm, Into2d, Into3d,
                    WithUniqueOrthogonal, Inverse, Rotate};

use glium::Surface;
use glium::backend::glutin_backend::GlutinFacade;

use ::{Batch, Scene, TextRenderer, RichText, Formatting, Font};

pub struct RenderContext {
    pub window: GlutinFacade,

    batch_program: glium::Program,
    text_renderer: TextRenderer,
}

impl RenderContext {
    #[allow(redundant_closure)]
    pub fn new(window: GlutinFacade) -> RenderContext {
        let dpi_factor = window.get_window().unwrap().hidpi_factor();

        RenderContext {
            batch_program: program!(&window, 140 => {
                vertex: include_str!("shader/solid_140.glslv"),
                fragment: include_str!("shader/solid_140.glslf")
            })
                .unwrap(),
            text_renderer: TextRenderer::new(&window, dpi_factor),
            window: window,
        }
    }

    pub fn submit(&mut self, scene: &Scene) {
        let mut target = self.window.draw();

        let view: [[f32; 4]; 4] =
            *Iso3::look_at_rh(&scene.eye.position, &scene.eye.target, &scene.eye.up)
                .to_homogeneous()
                .as_ref();
        let perspective: [[f32; 4]; 4] = *Persp3::new(target.get_dimensions().0 as f32 /
                                                      target.get_dimensions().1 as f32,
                                                      scene.eye.field_of_view,
                                                      0.1,
                                                      50000.0)
            .to_matrix()
            .as_ref();

        let uniforms = uniform! {
            view: view,
            perspective: perspective
        };

        let params = glium::DrawParameters {
            depth: glium::Depth {
                test: glium::draw_parameters::DepthTest::IfLess,
                write: true,
                ..Default::default()
            },
            ..Default::default()
        };

        let decal_params = glium::DrawParameters {
            depth: glium::Depth {
                test: glium::draw_parameters::DepthTest::Overwrite,
                write: false,
                ..Default::default()
            },
            ..Default::default()
        };

        // draw a frame
        target.clear_color_and_depth((1.0, 1.0, 1.0, 1.0), 1.0);

        let mut render_debug_text = String::from("Renderer:\n");

        let mut batches_todo = scene.batches.iter().collect::<Vec<_>>();
        batches_todo.sort_by_key(|&(batch_id, _)| batch_id);

        for (i, &Batch { ref vertices, ref indices, ref instances, is_decal, .. }) in batches_todo {
            if instances.len() > 1 {
                render_debug_text.push_str(&format!("batch{}: {} instances\n", i, instances.len()));
            }
            let instance_buffer = glium::VertexBuffer::new(&self.window, instances).unwrap();
            target.draw((vertices, instance_buffer.per_instance().unwrap()),
                      indices,
                      &self.batch_program,
                      &uniforms,
                      if is_decal { &decal_params } else { &params })
                .unwrap();
        }

        let screen = {
            let (w, h) = self.window.get_framebuffer_dimensions();
            (w as f32, h as f32)
        };

        let debug_text = self.create_debug_text(scene, &render_debug_text);
        self.text_renderer.render_text(screen, &self.window, &mut target, &[debug_text]);

        target.finish().unwrap();
    }

    fn create_debug_text(&self, scene: &Scene, render_debug_text: &str) -> RichText {
        let width = {
            let window = self.window.get_window().unwrap();
            window.get_inner_size_pixels().unwrap().0
        };

        let text = scene.persistent_debug_text
            .iter()
            .chain(scene.debug_text.iter())
            .map(|(key, &(ref text, _))| format!("{}:\n{}\n", key, text))
            .collect::<String>() + render_debug_text;
        let formatting = scene.persistent_debug_text
            .iter()
            .chain(scene.debug_text.iter())
            .map(|(key, &(ref text, ref color))| {
                Formatting {
                    len: key.len() + text.len() + 3,
                    font: Font::Debug,
                    width: width,
                    color: *color,
                }
            })
            .chain(Some(Formatting {
                len: render_debug_text.len(),
                font: Font::Debug,
                width: width,
                color: [0.0, 0.0, 0.0, 0.5],
            }))
            .collect::<Vec<_>>();
        RichText::new(&self.text_renderer, &text, &formatting)
    }
}
