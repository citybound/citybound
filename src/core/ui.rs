use ::monet::glium::{DisplayBuild, glutin};
use kay::{ActorSystem, ID, Individual, Recipient, Fate};
use descartes::{N, P2, P3, V3, Into2d, Shape};
use ::monet::{Renderer, Scene, GlutinFacade, MoveEye};
use ::monet::glium::glutin::{Event, MouseScrollDelta, ElementState, MouseButton};
use core::geometry::AnyShape;
use ::std::collections::HashMap;

pub struct UserInterface {
    interactables_2d: HashMap<ID, AnyShape>,
    interactables_3d: HashMap<ID, AnyShape>,
    cursor_2d: P2,
    cursor_3d: P3,
    drag_start_2d: Option<P2>,
    drag_start_3d: Option<P3>,
    hovered_interactable: Option<ID>,
    active_interactable: Option<ID>
}
impl Individual for UserInterface{}

impl UserInterface{
    fn new() -> UserInterface {
        UserInterface {
            interactables_2d: HashMap::new(),
            interactables_3d: HashMap::new(),
            cursor_2d: P2::new(0.0, 0.0),
            cursor_3d: P3::new(0.0, 0.0, 0.0),
            drag_start_2d: None,
            drag_start_3d: None,
            hovered_interactable: None,
            active_interactable: None
        }
    }
}

#[derive(Compact, Clone)]
pub enum Add{Interactable2d(ID, AnyShape), Interactable3d(ID, AnyShape)}

impl Recipient<Add> for UserInterface {
    fn receive(&mut self, msg: &Add) -> Fate {match *msg{
        Add::Interactable2d(_id, ref _shape) => unimplemented!(),
        Add::Interactable3d(id, ref shape) => {
            self.interactables_3d.insert(id, shape.clone());
            Fate::Live
        }
    }}
}

#[derive(Copy, Clone)]
pub enum Remove{Interactable2d(ID), Interactable3d(ID)}

impl Recipient<Remove> for UserInterface {
    fn receive(&mut self, msg: &Remove) -> Fate {match *msg{
        Remove::Interactable2d(_id) => unimplemented!(),
        Remove::Interactable3d(id) => {
            self.interactables_3d.remove(&id);
            Fate::Live
        }
    }}
}

#[derive(Copy, Clone)]
enum Mouse{Moved(P2), Down, Up}

use ::monet::Project2dTo3d;

#[derive(Copy, Clone)]
pub enum Dragging3d{Ongoing{from: P3, to: P3}, Finished, Aborted}

impl Recipient<Mouse> for UserInterface {
    fn receive(&mut self, msg: &Mouse) -> Fate {match *msg{
        Mouse::Moved(position) => {
            self.cursor_2d = position;
            Renderer::id() << Project2dTo3d{
                scene_id: 0,
                position_2d: position,
                requester: Self::id()
            };
            Fate::Live
        },
        Mouse::Down => {
            self.drag_start_2d = Some(self.cursor_2d);
            self.drag_start_3d = Some(self.cursor_3d);
            self.active_interactable = self.hovered_interactable;
            Fate::Live
        },
        Mouse::Up => {
            self.drag_start_2d = None;
            self.drag_start_3d = None;
            if let Some(active_interactable) = self.active_interactable {
                active_interactable << Dragging3d::Finished;
            }
            self.active_interactable = None;
            Fate::Live
        }
    }}
}

use ::monet::Projected3d;

impl Recipient<Projected3d> for UserInterface {
    fn receive(&mut self, msg: &Projected3d) -> Fate {match *msg{
        Projected3d{position_3d} => {
            self.cursor_3d = position_3d;
            if let Some(active_interactable) = self.active_interactable {
                active_interactable << Dragging3d::Ongoing{
                    from: self.drag_start_3d.expect("active interactable but no drag start"),
                    to: position_3d
                };
            } else {
                self.hovered_interactable = self.interactables_3d.iter().find(|&(_id, shape)|
                    shape.contains(position_3d.into_2d())
                ).map(|(id, _shape)| *id);
            }
            Fate::Live
        }
    }}
}

pub fn setup_window_and_renderer(system: &mut ActorSystem, renderables: Vec<ID>) -> GlutinFacade {
    let window = glutin::WindowBuilder::new()
        .with_title("Citybound".to_string())
        .with_dimensions(1024, 512)
        .with_multitouch()
        .with_vsync().build_glium().unwrap();

    let ui = UserInterface::new();

    system.add_individual(ui);
    system.add_inbox::<Add, UserInterface>();
    system.add_inbox::<Remove, UserInterface>();
    system.add_inbox::<Mouse, UserInterface>();
    system.add_inbox::<Projected3d, UserInterface>();

    let mut renderer = Renderer::new(window.clone());
    let mut scene = Scene::new();
    scene.eye.position *= 30.0;
    scene.renderables = renderables;
    renderer.scenes.insert(0, scene);

    ::monet::setup(system, renderer);

    window
}

pub fn process_events(window: &GlutinFacade) -> bool {
    for event in window.poll_events() {
        match event {
            Event::Closed => return false,
            Event::MouseWheel(MouseScrollDelta::PixelDelta(x, y), _) =>
                Renderer::id() << MoveEye{scene_id: 0, delta: V3::new(y / 5.0, -x / 5.0, 0.0)},
            Event::MouseMoved(x, y) =>
                UserInterface::id() << Mouse::Moved(P2::new(x as N, y as N)),
            Event::MouseInput(ElementState::Pressed, MouseButton::Left) =>
                UserInterface::id() << Mouse::Down,
            Event::MouseInput(ElementState::Released, MouseButton::Left) =>
                UserInterface::id() << Mouse::Up,
            _ => {}
        }
    }
    true
}