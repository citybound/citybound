#[macro_use]
extern crate glium;
extern crate glium_text;
extern crate nalgebra;
use nalgebra::{Point3, Vector3, Isometry3, Perspective3, ToHomogeneous};

use glium::glutin;
use glium::index::PrimitiveType;
use glium::Surface;

use std::sync::mpsc::{Sender, Receiver};

pub fn main_loop (prepare_frame: Sender<()>, data_provider: Receiver<String>) {
    use glium::DisplayBuild;
    
    let window = glutin::WindowBuilder::new()
        .with_title("Citybound".to_string())
        .with_dimensions(512, 512)
        .with_vsync().build_glium().unwrap();
        
    #[derive(Copy, Clone)]
    struct Vertex {
        position: [f32; 3]
    }

    implement_vertex!(Vertex, position);

    //          a simple car
    //
    //          .  B-------------C
    //      3-------------4  `    \               1.65
    //     /   9-A         \     .  D-------E     |
    //  1-2    |             5-------6  `   |     Z
    //  |   .  8- - - - - - - - - - -|------F     |    Y   .  0.9
    //  0----------------------------7  `         0    -0.9
    //
    // -2.25-----------X----------2.25
    
    let vertex_buffer = glium::VertexBuffer::new(&window, &[
        Vertex { position: [ -2.25,  -0.9, 0.00 ] }, // 0
        Vertex { position: [ -2.25,  -0.9, 0.80 ] }, // 1
        Vertex { position: [ -2.00,  -0.9, 1.00 ] }, // 2
        Vertex { position: [ -1.75,  -0.9, 1.65 ] }, // 3
        Vertex { position: [  0.30,  -0.9, 1.65 ] }, // 4
        Vertex { position: [  1.00,  -0.9, 1.00 ] }, // 5
        Vertex { position: [  2.25,  -0.9, 0.80 ] }, // 6
        Vertex { position: [  2.25,  -0.9, 0.00 ] }, // 7

        Vertex { position: [ -2.25,   0.9, 0.00 ] }, // 8
        Vertex { position: [ -2.25,   0.9, 0.80 ] }, // 9
        Vertex { position: [ -2.00,   0.9, 1.00 ] }, // A
        Vertex { position: [ -1.75,   0.9, 1.65 ] }, // B
        Vertex { position: [  0.30,   0.9, 1.65 ] }, // C
        Vertex { position: [  1.00,   0.9, 1.00 ] }, // D
        Vertex { position: [  2.25,   0.9, 0.80 ] }, // E
        Vertex { position: [  2.25,   0.9, 0.00 ] }, // F
    ]).unwrap();
        
    let index_buffer = glium::IndexBuffer::new(&window, PrimitiveType::TrianglesList, &[
        // right side
        0, 1, 2,
        0, 2, 5,
        0, 5, 7,
        5, 6, 7,
        2, 3, 4,
        2, 4, 5,
        // left side
        8, 9, 0xA,
        8, 0xA, 0xD,
        8, 0xD, 0xF,
        0xD, 0xE, 0xF,
        0xA, 0xB, 0xC,
        0xA, 0xC, 0xD,
        // connection between sides (front to back)
        8, 9, 1,
        8, 1, 0,
        9, 0xA, 2,
        9, 2, 1,
        0xA, 0xB, 3,
        0xA, 3, 2,
        0xB, 0xC, 4,
        0xB, 4, 3,
        0xC, 0xD, 5,
        0xC, 5, 4,
        0xD, 0xE, 6,
        0xD, 6, 5,
        0xE, 0xF, 7,
        0xE, 7, 6u16
    ]).unwrap();
    
    let program = program!(&window,
        140 => {
            vertex: "
                #version 140
                uniform mat4 model;
                uniform mat4 view;
                uniform mat4 perspective;
                in vec3 position;
                out vec3 p;
                void main() {
                    mat4 modelview = view * model;
                    gl_Position = perspective * modelview * vec4(position, 1.0);
                    p = position;
                }
            ",

            fragment: "
                #version 140
                out vec4 f_color;
                in vec3 p;
                void main() {
                    f_color = vec4(1.0, p.x, p.y, 1.0);
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

        let mut target = window.draw();

        let view : [[f32; 4]; 4] = *Isometry3::look_at_rh(
            &Point3::new(-5.0, -5.0, 5.0),
            &Point3::new(0.0, 0.0, 0.0),
            &Vector3::new(0.0, 0.0, 1.0)
        ).to_homogeneous().as_ref();
        let perspective : [[f32; 4]; 4] = *Perspective3::new(
            target.get_dimensions().0 as f32 / target.get_dimensions().1 as f32,
            0.3 * std::f32::consts::PI,
            0.1,
            1000.0
        ).to_matrix().as_ref();

        let model = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0f32]
        ];
        
        let uniforms = uniform! {
            model: model,
            view: view,
            perspective: perspective
        };

        let params = glium::DrawParameters {
            depth: glium::Depth {
                test: glium::draw_parameters::DepthTest::IfLess,
                write: true,
                .. Default::default()
            },
            backface_culling: glium::draw_parameters::BackfaceCullingMode::CullingDisabled,
            .. Default::default()
        };
        
        // draw a frame
        target.clear_color_and_depth((0.0, 0.0, 0.0, 0.0), 1.0);
        target.draw(&vertex_buffer, &index_buffer, &program, &uniforms, &params).unwrap();

        let text = glium_text::TextDisplay::new(&text_system, &font, new_data.as_str());
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