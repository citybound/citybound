use ::monet::glium::{DisplayBuild, glutin};
use kay::{ActorSystem, World, ID};
use ::monet::{Renderer, Scene, GlutinFacade, MoveEye, Vector3};
use ::monet::glium::glutin::{Event, MouseScrollDelta};

pub fn setup_window_and_renderer(system: &mut ActorSystem, renderables: Vec<ID>) -> GlutinFacade {
    let window = glutin::WindowBuilder::new()
        .with_title("Citybound".to_string())
        .with_dimensions(1024, 512)
        .with_multitouch()
        .with_vsync().build_glium().unwrap();

    let mut renderer = Renderer::new(window.clone());
    let mut scene = Scene::new();
    scene.eye.position *= 30.0;
    scene.renderables = renderables;
    renderer.scenes.insert(0, scene);

    ::monet::setup(system, renderer);

    window
}

pub fn process_events(window: &GlutinFacade, world: &mut World) -> bool {
    for event in window.poll_events() {
        match event {
            Event::Closed => return false,
            Event::MouseWheel(MouseScrollDelta::PixelDelta(x, y), _) =>
                world.send_to_individual::<Renderer, _>(MoveEye{scene_id: 0, delta: Vector3::<f32>::new(y / 5.0, -x / 5.0, 0.0)}),
            _ => {}
        }
    }
    true
}