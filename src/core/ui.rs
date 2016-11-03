use ::monet::glium::{DisplayBuild, glutin};
use kay::{ActorSystem, World, ID, Individual, Recipient};
use descartes::{N, P2, P3, V3};
use ::monet::{Renderer, Scene, GlutinFacade, MoveEye, Project2dTo3d, Projected3d, Thing,
    Vertex, SetupInScene, RenderToScene, AddBatch, AddInstance, Instance};
use ::monet::glium::glutin::{Event, MouseScrollDelta};

pub struct UserInterface {
    interactables_2d: Vec<ID>,
    interactables_3d: Vec<ID>,
    cursor_2d: P2,
    cursor_3d: P3
}
impl Individual for UserInterface{}

impl UserInterface{
    fn new() -> UserInterface {
        UserInterface {
            interactables_2d: Vec::new(),
            interactables_3d: Vec::new(),
            cursor_2d: P2::new(0.0, 0.0),
            cursor_3d: P3::new(0.0, 0.0, 0.0)
        }
    }
}

#[derive(Copy, Clone)]
struct MouseMoved(P2);

impl Recipient<MouseMoved> for UserInterface {
    fn react_to(&mut self, msg: &MouseMoved, world: &mut World, self_id: ID) {match *msg{
        MouseMoved(position) => {
            self.cursor_2d = position;
            world.send_to_individual::<Renderer, _>(Project2dTo3d{
                scene_id: 0,
                position_2d: position,
                requester: self_id
            });
            //println!("{}", position);
        }
    }}
}

impl Recipient<Projected3d> for UserInterface {
    fn receive(&mut self, msg: &Projected3d) {match *msg{
        Projected3d{position_3d} => {
            self.cursor_3d = position_3d;
            println!("3d pos: {:?}", position_3d);
        }
    }}
}

impl Recipient<SetupInScene> for UserInterface {
    fn react_to(&mut self, msg: &SetupInScene, world: &mut World, _self_id: ID) {match *msg{
        SetupInScene{renderer_id, scene_id} => {
            world.send(renderer_id, AddBatch::new(scene_id, 42, Thing::new(
                vec![
                    Vertex{position: [-5.0, -5.0, 0.0]},
                    Vertex{position: [5.0, -5.0, 0.0]},
                    Vertex{position: [5.0, 5.0, 0.0]},
                    Vertex{position: [-5.0, 5.0, 0.0]},
                ],
                vec![
                    0, 1, 2,
                    2, 3, 0
                ]
            )));
        }
    }}
}

impl Recipient<RenderToScene> for UserInterface {
    fn react_to(&mut self, msg: &RenderToScene, world: &mut World, _self_id: ID) {match *msg{
        RenderToScene{renderer_id, scene_id} => {
            world.send(renderer_id, AddInstance{
                scene_id: scene_id,
                batch_id: 42,
                position: Instance{
                    instance_position: *self.cursor_3d.as_ref(),
                    instance_direction: [1.0, 0.0],
                    instance_color: [0.0, 0.0, 1.0]
                }
            });
        }
    }}
}

pub fn setup_window_and_renderer(system: &mut ActorSystem, mut renderables: Vec<ID>) -> GlutinFacade {
    let window = glutin::WindowBuilder::new()
        .with_title("Citybound".to_string())
        .with_dimensions(1024, 512)
        .with_multitouch()
        .with_vsync().build_glium().unwrap();

    let ui = UserInterface::new();

    system.add_individual(ui);
    system.add_individual_inbox::<MouseMoved, UserInterface>();
    system.add_individual_inbox::<Projected3d, UserInterface>();
    system.add_individual_inbox::<SetupInScene, UserInterface>();
    system.add_individual_inbox::<RenderToScene, UserInterface>();

    let mut renderer = Renderer::new(window.clone());
    let mut scene = Scene::new();
    scene.eye.position *= 30.0;
    renderables.push(system.individual_id::<UserInterface>());
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
                world.send_to_individual::<Renderer, _>(MoveEye{scene_id: 0, delta: V3::new(y / 5.0, -x / 5.0, 0.0)}),
            Event::MouseMoved(x, y) =>
                world.send_to_individual::<UserInterface, _>(MouseMoved(P2::new(x as N, y as N))),
            _ => {}
        }
    }
    true
}