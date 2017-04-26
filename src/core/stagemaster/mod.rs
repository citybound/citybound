use kay::{ID, Actor, Recipient, Fate};
use descartes::{N, P2, V2, P3, Into2d, Shape};
use monet::{Renderer, Scene, GlutinFacade};
use monet::glium::glutin::{Event, MouseScrollDelta, ElementState, MouseButton};
pub use monet::glium::glutin::VirtualKeyCode;
use core::geometry::AnyShape;
use std::collections::HashMap;
use imgui::{ImGui, Ui, ImGuiSetCond_FirstUseEver, ImGuiKey};
use imgui::glium_renderer::Renderer as ImguiRenderer;
use std::collections::BTreeMap;

pub struct UserInterface {
    window: GlutinFacade,
    mouse_button_state: [bool; 5],
    cursor_2d: P2,
    cursor_3d: P3,
    drag_start_2d: Option<P2>,
    drag_start_3d: Option<P3>,
    interactables: HashMap<ID, (AnyShape, usize)>,
    hovered_interactable: Option<ID>,
    active_interactable: Option<ID>,
    focused_interactable: Option<ID>,
    imgui: ImGui,
    imgui_capture_keyboard: bool,
    imgui_capture_mouse: bool,
    imgui_renderer: ImguiRenderer,
    debug_text: BTreeMap<String, (String, [f32; 4])>,
    persistent_debug_text: BTreeMap<String, (String, [f32; 4])>,
}

impl Actor for UserInterface {}

impl UserInterface {
    fn new(window: GlutinFacade) -> Self {
        let mut imgui = ImGui::init();
        let imgui_renderer = ImguiRenderer::init(&mut imgui, &window).unwrap();

        imgui.set_imgui_key(ImGuiKey::Tab, 0);
        imgui.set_imgui_key(ImGuiKey::LeftArrow, 1);
        imgui.set_imgui_key(ImGuiKey::RightArrow, 2);
        imgui.set_imgui_key(ImGuiKey::UpArrow, 3);
        imgui.set_imgui_key(ImGuiKey::DownArrow, 4);
        imgui.set_imgui_key(ImGuiKey::PageUp, 5);
        imgui.set_imgui_key(ImGuiKey::PageDown, 6);
        imgui.set_imgui_key(ImGuiKey::Home, 7);
        imgui.set_imgui_key(ImGuiKey::End, 8);
        imgui.set_imgui_key(ImGuiKey::Delete, 9);
        imgui.set_imgui_key(ImGuiKey::Backspace, 10);
        imgui.set_imgui_key(ImGuiKey::Enter, 11);
        imgui.set_imgui_key(ImGuiKey::Escape, 12);
        imgui.set_imgui_key(ImGuiKey::A, 13);
        imgui.set_imgui_key(ImGuiKey::C, 14);
        imgui.set_imgui_key(ImGuiKey::V, 15);
        imgui.set_imgui_key(ImGuiKey::X, 16);
        imgui.set_imgui_key(ImGuiKey::Y, 17);
        imgui.set_imgui_key(ImGuiKey::Z, 18);

        UserInterface {
            window: window,
            mouse_button_state: [false; 5],
            cursor_2d: P2::new(0.0, 0.0),
            cursor_3d: P3::new(0.0, 0.0, 0.0),
            drag_start_2d: None,
            drag_start_3d: None,
            interactables: HashMap::new(),
            hovered_interactable: None,
            active_interactable: None,
            focused_interactable: None,
            imgui: imgui,
            imgui_capture_keyboard: false,
            imgui_capture_mouse: false,
            imgui_renderer: imgui_renderer,
            debug_text: BTreeMap::new(),
            persistent_debug_text: BTreeMap::new(),
        }
    }
}

#[derive(Copy, Clone)]
pub struct ProcessEvents;

impl Recipient<ProcessEvents> for UserInterface {
    fn receive(&mut self, _msg: &ProcessEvents) -> Fate {
        let scale = self.imgui.display_framebuffer_scale();

        for event in self.window.poll_events().collect::<Vec<_>>() {
            match event {
                Event::Closed => ::std::process::exit(0),

                Event::MouseWheel(delta, _) => {
                    let v = match delta {
                        MouseScrollDelta::LineDelta(x, y) => V2::new(x * 50.0 as N, y * 50.0 as N),
                        MouseScrollDelta::PixelDelta(x, y) => V2::new(x as N, y as N),
                    };

                    self.imgui.set_mouse_wheel(v.y / scale.1);

                    if !self.imgui_capture_mouse {
                        self.focused_interactable
                            .map(|interactable| interactable << Event3d::Scroll(v));
                    }
                }
                Event::MouseMoved(x, y) => {
                    self.cursor_2d = P2::new(x as N, y as N);

                    self.imgui
                        .set_mouse_pos(self.cursor_2d.x / scale.0, self.cursor_2d.y / scale.1);

                    Renderer::id() <<
                    Project2dTo3d {
                        scene_id: 0,
                        position_2d: self.cursor_2d,
                        requester: Self::id(),
                    };
                }
                Event::MouseInput(button_state, button) => {
                    let button_idx = match button {
                        MouseButton::Left => 0,
                        MouseButton::Right => 1,
                        MouseButton::Middle => 2,
                        _ => 4,
                    };
                    let pressed = button_state == ElementState::Pressed;
                    self.mouse_button_state[button_idx] = pressed;

                    self.imgui.set_mouse_down(&self.mouse_button_state);

                    if !self.imgui_capture_mouse {
                        if pressed {
                            self.drag_start_2d = Some(self.cursor_2d);
                            self.drag_start_3d = Some(self.cursor_3d);
                            let cursor_3d = self.cursor_3d;
                            self.receive(&Projected3d { position_3d: cursor_3d });
                            self.active_interactable = self.hovered_interactable;
                            if let Some(active_interactable) = self.active_interactable {
                                active_interactable <<
                                Event3d::DragStarted {
                                    at: self.cursor_3d,
                                    at2d: self.cursor_2d,
                                };
                            }
                        } else {
                            if let Some(active_interactable) = self.active_interactable {
                                active_interactable <<
                                Event3d::DragFinished {
                                    from: self.drag_start_3d
                                        .expect("active interactable but no drag start"),
                                    from2d: self.drag_start_2d
                                        .expect("active interactable but no drag start"),
                                    to: self.cursor_3d,
                                    to2d: self.cursor_2d,
                                };
                            }
                            self.drag_start_2d = None;
                            self.drag_start_3d = None;
                            self.active_interactable = None;
                        }
                    }
                }
                Event::KeyboardInput(button_state, _, Some(key_code)) => {
                    let pressed = button_state == ElementState::Pressed;

                    if self.imgui_capture_keyboard {
                        match key_code {
                            VirtualKeyCode::Tab => self.imgui.set_key(0, pressed),
                            VirtualKeyCode::Left => self.imgui.set_key(1, pressed),
                            VirtualKeyCode::Right => self.imgui.set_key(2, pressed),
                            VirtualKeyCode::Up => self.imgui.set_key(3, pressed),
                            VirtualKeyCode::Down => self.imgui.set_key(4, pressed),
                            VirtualKeyCode::PageUp => self.imgui.set_key(5, pressed),
                            VirtualKeyCode::PageDown => self.imgui.set_key(6, pressed),
                            VirtualKeyCode::Home => self.imgui.set_key(7, pressed),
                            VirtualKeyCode::End => self.imgui.set_key(8, pressed),
                            VirtualKeyCode::Delete => self.imgui.set_key(9, pressed),
                            VirtualKeyCode::Back => self.imgui.set_key(10, pressed),
                            VirtualKeyCode::Return => self.imgui.set_key(11, pressed),
                            VirtualKeyCode::Escape => self.imgui.set_key(12, pressed),
                            VirtualKeyCode::A => self.imgui.set_key(13, pressed),
                            VirtualKeyCode::C => self.imgui.set_key(14, pressed),
                            VirtualKeyCode::V => self.imgui.set_key(15, pressed),
                            VirtualKeyCode::X => self.imgui.set_key(16, pressed),
                            VirtualKeyCode::Y => self.imgui.set_key(17, pressed),
                            VirtualKeyCode::Z => self.imgui.set_key(18, pressed),
                            VirtualKeyCode::LControl | VirtualKeyCode::RControl => {
                                self.imgui.set_key_ctrl(pressed)
                            }
                            VirtualKeyCode::LShift | VirtualKeyCode::RShift => {
                                self.imgui.set_key_shift(pressed)
                            }
                            VirtualKeyCode::LAlt | VirtualKeyCode::RAlt => {
                                self.imgui.set_key_alt(pressed)
                            }
                            VirtualKeyCode::LWin | VirtualKeyCode::RWin => {
                                self.imgui.set_key_super(pressed)
                            }
                            _ => {}
                        }
                    } else {
                        self.focused_interactable.map(|interactable| {
                            interactable <<
                            if pressed {
                                Event3d::KeyDown(key_code)
                            } else {
                                Event3d::KeyUp(key_code)
                            }
                        });
                    }
                }
                Event::ReceivedCharacter(c) => self.imgui.add_input_character(c),
                _ => {}
            }
        }

        Fate::Live
    }
}

#[derive(Compact, Clone)]
pub struct AddInteractable(pub ID, pub AnyShape, pub usize);

impl Recipient<AddInteractable> for UserInterface {
    fn receive(&mut self, msg: &AddInteractable) -> Fate {
        match *msg {
            AddInteractable(id, ref shape, z_index) => {
                self.interactables.insert(id, (shape.clone(), z_index));
                Fate::Live
            }
        }
    }
}

#[derive(Copy, Clone)]
pub struct RemoveInteractable(pub ID);

impl Recipient<RemoveInteractable> for UserInterface {
    fn receive(&mut self, msg: &RemoveInteractable) -> Fate {
        match *msg {
            RemoveInteractable(id) => {
                self.interactables.remove(&id);
                Fate::Live
            }
        }
    }
}

#[derive(Copy, Clone)]
pub struct Focus(pub ID);

impl Recipient<Focus> for UserInterface {
    fn receive(&mut self, msg: &Focus) -> Fate {
        match *msg {
            Focus(id) => {
                self.focused_interactable = Some(id);
                Fate::Live
            }
        }
    }
}

use monet::Project2dTo3d;

#[derive(Copy, Clone)]
pub enum Event3d {
    DragStarted { at: P3, at2d: P2 },
    DragOngoing {
        from: P3,
        from2d: P2,
        to: P3,
        to2d: P2,
    },
    DragFinished {
        from: P3,
        from2d: P2,
        to: P3,
        to2d: P2,
    },
    DragAborted,
    HoverStarted { at: P3, at2d: P2 },
    HoverOngoing { at: P3, at2d: P2 },
    HoverStopped,
    Scroll(V2),
    KeyDown(VirtualKeyCode),
    KeyUp(VirtualKeyCode),
}

use monet::Projected3d;

impl Recipient<Projected3d> for UserInterface {
    fn receive(&mut self, msg: &Projected3d) -> Fate {
        match *msg {
            Projected3d { position_3d } => {
                self.cursor_3d = position_3d;
                if let Some(active_interactable) = self.active_interactable {
                    active_interactable <<
                    Event3d::DragOngoing {
                        from: self.drag_start_3d.expect("active interactable but no drag start"),
                        from2d: self.drag_start_2d.expect("active interactable but no drag start"),
                        to: position_3d,
                        to2d: self.cursor_2d,
                    };
                } else {
                    let new_hovered_interactable = self.interactables
                        .iter()
                        .filter(|&(_id, &(ref shape, _z_index))| {
                            shape.contains(position_3d.into_2d())
                        })
                        .max_by_key(|&(_id, &(ref _shape, z_index))| z_index)
                        .map(|(id, _shape)| *id);

                    if self.hovered_interactable != new_hovered_interactable {
                        if let Some(previous) = self.hovered_interactable {
                            previous << Event3d::HoverStopped;
                        }
                        if let Some(next) = new_hovered_interactable {
                            next <<
                            Event3d::HoverStarted {
                                at: self.cursor_3d,
                                at2d: self.cursor_2d,
                            };
                        }
                    } else if let Some(hovered_interactable) = self.hovered_interactable {
                        hovered_interactable <<
                        Event3d::HoverOngoing {
                            at: self.cursor_3d,
                            at2d: self.cursor_2d,
                        };
                    }
                    self.hovered_interactable = new_hovered_interactable;
                }
                Fate::Live
            }
        }
    }
}

#[derive(Copy, Clone)]
pub struct StartFrame;

impl Recipient<StartFrame> for UserInterface {
    fn receive(&mut self, _msg: &StartFrame) -> Fate {
        let target = Box::new(self.window.draw());

        let target_ptr = Box::into_raw(target);

        Renderer::id() <<
        ::monet::Control::Submit {
            target_ptr: target_ptr as usize,
            return_to: Self::id(),
        };

        Fate::Live
    }
}

use monet::Submitted;

impl Recipient<Submitted> for UserInterface {
    fn receive(&mut self, msg: &Submitted) -> Fate {
        match *msg {
            Submitted { target_ptr } => {
                let mut target = unsafe { Box::from_raw(target_ptr as *mut ::monet::glium::Frame) };

                let size_points =
                    self.window.get_window().unwrap().get_inner_size_points().unwrap();
                let size_pixels =
                    self.window.get_window().unwrap().get_inner_size_pixels().unwrap();
                let ui = self.imgui.frame(size_points, size_pixels, 1.0 / 60.0);

                self.imgui_capture_keyboard = ui.want_capture_keyboard();
                self.imgui_capture_mouse = ui.want_capture_mouse();

                let texts: Vec<_> =
                    self.persistent_debug_text.iter().chain(self.debug_text.iter()).collect();

                ui.window(im_str!("Debug Info"))
                    .size((600.0, 200.0), ImGuiSetCond_FirstUseEver)
                    .build(|| for (key, &(ref text, ref color)) in texts {
                        ui.text_colored(*color, im_str!("{}:\n{}", key, text));
                    });

                self.imgui_renderer.render(&mut *target, ui).unwrap();

                target.finish().unwrap();

                Fate::Live
            }
        }
    }
}

use compact::CVec;

#[derive(Compact, Clone)]
pub struct AddDebugText {
    pub key: CVec<char>,
    pub text: CVec<char>,
    pub color: [f32; 4],
    pub persistent: bool,
}

impl Recipient<AddDebugText> for UserInterface {
    fn receive(&mut self, msg: &AddDebugText) -> Fate {
        match *msg {
            AddDebugText { ref key, ref text, ref color, persistent } => {
                let target = if persistent {
                    &mut self.persistent_debug_text
                } else {
                    &mut self.debug_text
                };
                target.insert(key.iter().cloned().collect(),
                              (text.iter().cloned().collect(), *color));
                Fate::Live
            }
        }
    }
}

pub fn setup(renderables: Vec<ID>, window: &GlutinFacade) {
    let mut renderer = Renderer::new(window.clone());
    let mut scene = Scene::new();
    scene.eye.position *= 30.0;
    scene.renderables = renderables;
    renderer.scenes.insert(0, scene);

    ::monet::setup(renderer);

    UserInterface::register_with_state(UserInterface::new(window.clone()));
    UserInterface::handle::<AddInteractable>();
    UserInterface::handle::<RemoveInteractable>();
    UserInterface::handle::<Focus>();
    UserInterface::handle_critically::<ProcessEvents>();
    UserInterface::handle_critically::<StartFrame>();
    UserInterface::handle_critically::<Projected3d>();
    UserInterface::handle_critically::<Submitted>();
    UserInterface::handle_critically::<AddDebugText>();
}