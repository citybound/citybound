use ::monet::glium::{DisplayBuild, glutin};
use kay::{ActorSystem};
use ::monet::{Renderer, Scene, GlutinFacade};
use ::monet::glium::glutin::{Event};

pub fn setup_window_and_renderer(system: &mut ActorSystem) -> GlutinFacade {
    let window = glutin::WindowBuilder::new()
        .with_title("Citybound".to_string())
        .with_dimensions(512, 512)
        .with_multitouch()
        .with_vsync().build_glium().unwrap();

    let mut renderer = Renderer::new(window.clone());
    let mut scene = Scene::new();
    scene.eye.position *= 30.0;
    scene.renderables.push(system.broadcast_id::<::game::lanes_and_cars::Lane>());
    scene.renderables.push(system.broadcast_id::<::game::lanes_and_cars::TransferLane>());
    renderer.scenes.insert(0, scene);

    ::monet::setup(system, renderer);

    window
}

pub fn process_events(window: &GlutinFacade) -> bool {
    for event in window.poll_events() {
        match event {
            Event::Closed => return false,
            _ => {}
        }
    }
    true
}