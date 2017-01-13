use super::ui::{UserInterface, VirtualKeyCode, KeyOrButton, Mouse, KeyCombination,
                InterchangeableKeys};
use super::settings::{Settings, SETTINGS};
use ::monet::glium::glutin::MouseButton;
use kay::{Swarm, ToRandom, Recipient, ActorSystem, Individual, Fate};
use monet::Renderer;
use descartes::{P3, V3};

struct Camera;

impl Camera {
    fn new() -> Camera {
        ()
    }
}

impl Individual for Camera {}

use super::ui::UIInput;
use ::monet::MoveEye;

impl Recipient<UIInput> for Camera {
    fn receive(&mut self, msg: &UIInput) -> Fate {
        let inverted = if (*SETTINGS).invert_y {
            -1.0f32
        } else {
            1.0f32
        };
        for (action, name) in msg.mouse_events.into_iter() {
            match (action, name) {
                (Mouse::Moved(p), "Pan") => {
                    Renderer::id() <<
                    MoveEye {
                        scene_id: 0,
                        movement: ::monet::Movement::Shift(V3::new(-p.y * (*SETTINGS).move_speed *
                                                                   inverted /
                                                                   3.0,
                                                                   p.x * (*SETTINGS).move_speed *
                                                                   inverted /
                                                                   3.0,
                                                                   0.0)),
                    };
                }
                (Mouse::Moved(p), "Pitch") => {
                    Renderer::id() <<
                    MoveEye {
                        scene_id: 0,
                        movement: ::monet::Movement::Pitch(-p.y * (*SETTINGS).rotation_speed *
                                                           inverted /
                                                           300.0),
                    };
                }
                (Mouse::Moved(p), "Yaw") => {
                    Renderer::id() <<
                    MoveEye {
                        scene_id: 0,
                        movement: ::monet::Movement::Shift(V3::new(-p.y * (*SETTINGS).move_speed *
                                                                   inverted /
                                                                   3.0,
                                                                   p.x * (*SETTINGS).move_speed *
                                                                   inverted /
                                                                   3.0,
                                                                   0.0)),
                    };
                }
                (Mouse::Scrolled(p), "Zoom") => {
                    Renderer::id() <<
                    MoveEye {
                        scene_id: 0,
                        movement: ::monet::Movement::Zoom(p.y * (*SETTINGS).zoom_speed),
                    };
                }
                _ => (),
            }
        }
        for name in msg.button_events.into_iter() {
            if name == "Forwards" {
                Renderer::id() <<
                MoveEye {
                    scene_id: 0,
                    movement: ::monet::Movement::Shift(V3::new(5.0 * (*SETTINGS).move_speed,
                                                               0.0,
                                                               0.0)),
                };
            }
            if name == "Backwards" {
                Renderer::id() <<
                MoveEye {
                    scene_id: 0,
                    movement: ::monet::Movement::Shift(V3::new(-5.0 * (*SETTINGS).move_speed,
                                                               0.0,
                                                               0.0)),
                };
            }
            if name == "Left" {
                Renderer::id() <<
                MoveEye {
                    scene_id: 0,
                    movement: ::monet::Movement::Shift(V3::new(0.0,
                                                               -5.0 * (*SETTINGS).move_speed,
                                                               0.0)),
                };
            }
            if name == "Right" {
                Renderer::id() <<
                MoveEye {
                    scene_id: 0,
                    movement: ::monet::Movement::Shift(V3::new(0.0,
                                                               5.0 * (*SETTINGS).move_speed,
                                                               0.0)),
                };
            }
        }
        Fate::Live
    }
}

pub fn setup(system: &mut ActorSystem, settings: &mut Settings) {
    system.add_individual(Camera::new());
    system.add_inbox::<UIInput, Camera>();

    settings.register_key(KeyCombination {
                              keys: vec![InterchangeableKeys {
                                             keys: vec![KeyOrButton::Key(VirtualKeyCode::W),
                                                        KeyOrButton::Key(VirtualKeyCode::Up)],
                                         }],
                          },
                          "Forwards");

    settings.register_key(KeyCombination {
                              keys: vec![InterchangeableKeys {
                                             keys: vec![KeyOrButton::Key(VirtualKeyCode::S),
                                                        KeyOrButton::Key(VirtualKeyCode::Down)],
                                         }],
                          },
                          "Backwards");

    settings.register_key(KeyCombination {
                              keys: vec![InterchangeableKeys {
                                             keys: vec![KeyOrButton::Key(VirtualKeyCode::A),
                                                        KeyOrButton::Key(VirtualKeyCode::Left)],
                                         }],
                          },
                          "Left");

    settings.register_key(KeyCombination {
                              keys: vec![InterchangeableKeys {
                                             keys: vec![KeyOrButton::Key(VirtualKeyCode::D),
                                                        KeyOrButton::Key(VirtualKeyCode::Right)],
                                         }],
                          },
                          "Right");

    settings.register_mouse_modifier(KeyCombination {
                                         keys: vec![
                InterchangeableKeys{
                    keys: vec![KeyOrButton::Key(VirtualKeyCode::LShift),
                               KeyOrButton::Key(VirtualKeyCode::RShift)]
                },
            ],
                                     },
                                     "Pan");

    settings.register_mouse_modifier(KeyCombination {
                                         keys: vec![
                InterchangeableKeys{
                    keys: vec![KeyOrButton::Key(VirtualKeyCode::LAlt),
                               KeyOrButton::Key(VirtualKeyCode::RAlt),
                               KeyOrButton::Button(MouseButton::Middle)]
                },
            ],
                                     },
                                     "Pitch");

    settings.register_mouse_modifier(KeyCombination {
                                         keys: vec![
                InterchangeableKeys{
                    keys: vec![KeyOrButton::Key(VirtualKeyCode::LAlt),
                               KeyOrButton::Key(VirtualKeyCode::RAlt),
                                                KeyOrButton::Button(MouseButton::Middle)]
                },
            ],
                                     },
                                     "Yaw");
}
