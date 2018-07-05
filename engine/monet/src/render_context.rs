use glium;

pub use descartes::{N, P3, P2, V3, V4, M4, Iso3, Persp3, Into2d, Into3d, WithUniqueOrthogonal};

use glium::Surface;
use glium::backend::glutin::Display;
use kay::External;

use std::net::{TcpListener, TcpStream};
use tungstenite::WebSocket;
use byteorder::{LittleEndian, WriteBytesExt};

use {Batch, Scene};

pub struct RenderContext {
    pub window: External<Display>,
    pub websocket: WebSocket<TcpStream>,
    batch_program: glium::Program,
    clear_color: (f32, f32, f32, f32),
}

impl RenderContext {
    #[cfg_attr(feature = "cargo-clippy", allow(redundant_closure))]
    pub fn new(window: &External<Display>, clear_color: (f32, f32, f32, f32)) -> RenderContext {
        let tcp_listener = TcpListener::bind("127.0.0.1:9999").unwrap();
        println!("Awaiting TCP connection");
        let websocket = ::tungstenite::server::accept(tcp_listener.accept().unwrap().0).unwrap();
        RenderContext {
            batch_program: program!(&**window, 140 => {
                vertex: include_str!("shader/solid_140.glslv"),
                fragment: include_str!("shader/solid_140.glslf")
            }).unwrap(),
            websocket,
            window: window.steal(),
            clear_color,
        }
    }

    pub fn submit<S: Surface>(&mut self, scene: &Scene, target: &mut S) {
        let view: [[f32; 4]; 4] =
            *Iso3::look_at_rh(&scene.eye.position, &scene.eye.target, &scene.eye.up)
                .to_homogeneous()
                .as_ref();
        let perspective: [[f32; 4]; 4] = *Persp3::new(
            target.get_dimensions().0 as f32 / target.get_dimensions().1 as f32,
            scene.eye.field_of_view,
            0.1,
            50000.0,
        ).as_matrix()
            .as_ref();

        let mut websocket_message =
            Vec::<u8>::with_capacity(4 + 2 * 4 * 4 * ::std::mem::size_of::<f32>());

        // frame start
        websocket_message.write_u32::<LittleEndian>(0).unwrap();

        websocket_message.resize(4 + 2 * 4 * ::std::mem::size_of::<[f32; 4]>(), 0);
        unsafe {
            view.as_ptr()
                .copy_to(&mut websocket_message[4] as *mut u8 as *mut [f32; 4], 4)
        }
        unsafe {
            perspective.as_ptr().copy_to(
                &mut websocket_message[4 + 4 * ::std::mem::size_of::<[f32; 4]>()] as *mut u8
                    as *mut [f32; 4],
                4,
            )
        }

        self.websocket
            .write_message(::tungstenite::Message::binary(websocket_message))
            .unwrap();

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
        target.clear_color_and_depth(self.clear_color, 1.0);

        let mut render_debug_text = String::from("Renderer:\n");

        let mut batches_todo = scene.batches.iter().collect::<Vec<_>>();
        batches_todo.sort_by_key(|&(batch_id, _)| batch_id);

        for (
            i,
            &Batch {
                ref vertices,
                ref indices,
                ref instances,
                is_decal,
                full_frame_instance_end,
                ..
            },
        ) in batches_todo
        {
            let instances_to_draw =
                &instances[..full_frame_instance_end.unwrap_or_else(|| instances.len())];
            if instances_to_draw.len() > 1 {
                render_debug_text.push_str(&format!(
                    "batch{}: {} instances\n",
                    i,
                    instances_to_draw.len()
                ));
            }

            // drawcall
            if !instances_to_draw.is_empty() {
                let mut websocket_message = Vec::<u8>::new();
                websocket_message.write_u32::<LittleEndian>(42).unwrap();
                websocket_message.write_u32::<LittleEndian>(*i).unwrap();
                websocket_message
                    .write_u32::<LittleEndian>(instances_to_draw.len() as u32)
                    .unwrap();
                let instances_pos = websocket_message.len();
                websocket_message.resize(
                    instances_pos + instances.len() * ::std::mem::size_of::<::mesh::Instance>(),
                    0,
                );
                unsafe {
                    instances_to_draw.as_ptr().copy_to(
                        &mut websocket_message[instances_pos] as *mut u8 as *mut ::mesh::Instance,
                        instances_to_draw.len(),
                    )
                }
                self.websocket
                    .write_message(::tungstenite::Message::binary(websocket_message))
                    .unwrap();
            }

            let instance_buffer =
                glium::VertexBuffer::new(&*self.window, instances_to_draw).unwrap();
            target
                .draw(
                    (vertices, instance_buffer.per_instance().unwrap()),
                    indices,
                    &self.batch_program,
                    &uniforms,
                    if is_decal { &decal_params } else { &params },
                )
                .unwrap();
        }

        // let size_points = self.window.get_window().unwrap().get_inner_size_points().unwrap();
        // let size_pixels = self.window.get_window().unwrap().get_inner_size_pixels().unwrap();
        // let ui = self.imgui.frame(size_points, size_pixels, 1.0 / 60.0);

        // ui.window(im_str!("Debug Info"))
        //     .size((600.0, 200.0), ImGuiSetCond_FirstUseEver)
        //     .build(|| for (key, &(ref text, ref color)) in
        //         scene.persistent_debug_text.iter().chain(scene.debug_text.iter()) {
        //         ui.text_colored(*color, im_str!("{}:\n{}", key, text));
        //     });

        // self.imgui_renderer.render(target, ui).unwrap();
    }
}
