#[macro_use]
extern crate glium;
extern crate glium_text;
use glium::glutin;
use glium::index::PrimitiveType;
use glium::Surface;

use std::sync::mpsc::{Sender, Receiver};

pub fn main_loop (prepare_frame: Sender<()>, data_provider: Receiver<()>) {
    use glium::DisplayBuild;
    
    let window = glutin::WindowBuilder::new()
        .with_title("Citybound".to_string())
        .with_dimensions(512, 512)
        .with_vsync().build_glium().unwrap();
        
    #[derive(Copy, Clone)]
    struct Vertex {
        position: [f32; 2],
        color: [f32; 3],
    }

    implement_vertex!(Vertex, position, color);
    
    let vertex_buffer = glium::VertexBuffer::new(&window, &[
        Vertex { position: [-0.5, -0.5], color: [0.0, 1.0, 0.0] },
        Vertex { position: [ 0.0,  0.5], color: [0.0, 0.0, 1.0] },
        Vertex { position: [ 0.5, -0.5], color: [1.0, 0.0, 0.0] },
    ]).unwrap();
        
    let index_buffer = glium::IndexBuffer::new(&window, PrimitiveType::TrianglesList, &[0u16, 1, 2]).unwrap();
    
    let program = program!(&window,
        140 => {
            vertex: "
                #version 140
                uniform mat4 matrix;
                in vec2 position;
                in vec3 color;
                out vec3 vColor;
                void main() {
                    gl_Position = vec4(position, 0.0, 1.0) * matrix;
                    vColor = color;
                }
            ",

            fragment: "
                #version 140
                in vec3 vColor;
                out vec4 f_color;
                void main() {
                    f_color = vec4(vColor, 1.0);
                }
            "
        },
    ).unwrap();

    let text_system = glium_text::TextSystem::new(&window);
    let font = glium_text::FontTexture::new(
        &window,
        std::fs::File::open(&std::path::Path::new("resources/ClearSans-Regular.ttf")).unwrap(),
        64
    ).unwrap();

    'main: loop {
        // loop over events
        for event in window.poll_events() {
            match event {
                glutin::Event::KeyboardInput(_, _, Some(glutin::VirtualKeyCode::Escape)) |
                glutin::Event::Closed => break 'main,
                _ => {},
            }
        }
        
        prepare_frame.send(()).unwrap();
        let new_data = data_provider.recv().unwrap();
        println!("rendering...");

        let matrix = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0f32]
        ];
        
        let uniforms = uniform! {
            matrix: matrix
        };
        
        // draw a frame
        let mut target = window.draw();
        target.clear_color(0.0, 0.0, 0.0, 0.0);
        target.draw(&vertex_buffer, &index_buffer, &program, &uniforms, &Default::default()).unwrap();

        let text = glium_text::TextDisplay::new(&text_system, &font, "The city sim you deserve.");
        let text_matrix = [
            [0.05, 0.0, 0.0, 0.0],
            [0.0, 0.05, 0.0, 0.0],
            [0.0, 0.0, 0.05, 0.0],
            [-0.9, 0.8, 0.0, 1.0f32]
        ];

        glium_text::draw(&text, &text_system, &mut target, text_matrix, (1.0, 1.0, 0.0, 1.0));

        target.finish().unwrap();
        
    }
}