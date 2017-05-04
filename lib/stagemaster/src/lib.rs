#![feature(plugin)]
#![plugin(clippy)]
#![allow(no_effect, unnecessary_operation)]

extern crate compact;
#[macro_use]
extern crate compact_macros;
extern crate kay;
extern crate monet;
extern crate descartes;
#[macro_use]
extern crate imgui;
extern crate imgui_sys;

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate app_dirs;

pub mod geometry;
pub mod environment;
pub mod combo;
pub mod camera_control;

use kay::{ID, Actor, Recipient, Fate};
use descartes::{N, P2, V2, P3, Into2d, Shape};
use monet::{Renderer, Scene, GlutinFacade};
use monet::glium::glutin::{Event, MouseScrollDelta, ElementState, MouseButton};
pub use monet::glium::glutin::VirtualKeyCode;
use geometry::AnyShape;
use std::collections::{HashMap, HashSet};
use imgui::{ImGui, ImVec4, ImGuiSetCond_FirstUseEver, ImGuiKey};
use imgui_sys::{ImFontConfig, ImGuiCol, ImGuiAlign_Center, ImFontConfig_DefaultConstructor};
use imgui::glium_renderer::Renderer as ImguiRenderer;
use std::collections::BTreeMap;

pub struct UserInterface {
    window: GlutinFacade,
    mouse_button_state: [bool; 5],
    combo_listener: combo::ComboListener,
    cursor_2d: P2,
    cursor_3d: P3,
    drag_start_2d: Option<P2>,
    drag_start_3d: Option<P3>,
    interactables: HashMap<ID, (AnyShape, usize)>,
    hovered_interactable: Option<ID>,
    active_interactable: Option<ID>,
    interactables_2d: Vec<ID>,
    interactables_2d_todo: Vec<ID>,
    focused_interactables: HashSet<ID>,
    parked_frame: Option<Box<::monet::glium::Frame>>,
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
        let default_font = im_str!("fonts/ClearSans-Regular.ttf\0");

        unsafe {
            let atlas = (*imgui_sys::igGetIO()).fonts;
            let mut config: ImFontConfig = ::std::mem::zeroed();
            ImFontConfig_DefaultConstructor(&mut config);
            config.oversample_h = 2;
            config.oversample_v = 2;
            imgui_sys::ImFontAtlas_AddFontFromFileTTF(atlas,
                                                      default_font.as_ptr(),
                                                      16.0,
                                                      &config,
                                                      ::std::ptr::null());

            let style = imgui.style_mut();
            style.window_rounding = 4.0;
            style.grab_rounding = 3.0;
            style.scrollbar_rounding = 3.0;
            style.frame_rounding = 3.0;
            style.scrollbar_size = 14.0;
            style.window_title_align = ImGuiAlign_Center;
            style.colors[ImGuiCol::WindowBg as usize] = ImVec4::new(0.9, 0.9, 0.9, 0.8);
            style.colors[ImGuiCol::FrameBg as usize] = ImVec4::new(0.0, 0.0, 0.0, 0.2);
            style.colors[ImGuiCol::Text as usize] = ImVec4::new(0.0, 0.0, 0.0, 0.9);
            style.colors[ImGuiCol::TextDisabled as usize] = ImVec4::new(0.0, 0.0, 0.0, 0.5);
            style.colors[ImGuiCol::TitleBg as usize] = ImVec4::new(0.8, 0.8, 0.8, 0.9);
            style.colors[ImGuiCol::TitleBgActive as usize] = ImVec4::new(0.7, 0.7, 0.7, 1.0);
            style.colors[ImGuiCol::ScrollbarBg as usize] = ImVec4::new(0.0, 0.0, 0.0, 0.03);
            style.colors[ImGuiCol::ScrollbarGrab as usize] = ImVec4::new(0.0, 0.0, 0.0, 0.2);
            style.colors[ImGuiCol::ScrollbarGrabHovered as usize] = ImVec4::new(0.0, 0.0, 1.0, 0.6);
            style.colors[ImGuiCol::ScrollbarGrabActive as usize] = ImVec4::new(0.0, 0.0, 1.0, 1.0);
            style.colors[ImGuiCol::ResizeGrip as usize] = ImVec4::new(0.0, 0.0, 0.0, 0.1);
            style.colors[ImGuiCol::ResizeGripHovered as usize] = ImVec4::new(0.0, 0.0, 1.0, 0.6);
            style.colors[ImGuiCol::ResizeGripActive as usize] = ImVec4::new(0.0, 0.0, 1.0, 1.0);
            style.colors[ImGuiCol::ButtonHovered as usize] = ImVec4::new(0.0, 0.0, 1.0, 0.6);
            style.colors[ImGuiCol::ButtonActive as usize] = ImVec4::new(0.0, 0.0, 1.0, 1.0);
            style.colors[ImGuiCol::SliderGrab as usize] = ImVec4::new(0.0, 0.0, 0.0, 0.6);
            style.colors[ImGuiCol::SliderGrabActive as usize] = ImVec4::new(0.0, 0.0, 1.0, 1.0);
        }

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
            combo_listener: combo::ComboListener::default(),
            cursor_2d: P2::new(0.0, 0.0),
            cursor_3d: P3::new(0.0, 0.0, 0.0),
            drag_start_2d: None,
            drag_start_3d: None,
            interactables: HashMap::new(),
            hovered_interactable: None,
            active_interactable: None,
            focused_interactables: HashSet::new(),
            interactables_2d: Vec::new(),
            interactables_2d_todo: Vec::new(),
            parked_frame: None,
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

                    self.imgui.set_mouse_wheel(v.y / (scale.1 * 50.0));

                    if !self.imgui_capture_mouse {
                        for interactable in &self.focused_interactables {
                            *interactable << Event3d::Scroll(v)
                        }
                    }
                }
                Event::MouseMoved(x, y) => {
                    self.cursor_2d = P2::new(x as N, y as N);

                    self.imgui
                        .set_mouse_pos(self.cursor_2d.x / scale.0, self.cursor_2d.y / scale.1);

                    for interactable in &self.focused_interactables {
                        *interactable << Event3d::MouseMove(self.cursor_2d);
                    }

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
                        self.combo_listener.update(&event);

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

                        for interactable in &self.focused_interactables {
                            *interactable <<
                            if pressed {
                                Event3d::ButtonDown(button.into())
                            } else {
                                Event3d::ButtonUp(button.into())
                            };

                            *interactable << Event3d::Combos(self.combo_listener);
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
                        self.combo_listener.update(&event);

                        for interactable in &self.focused_interactables {
                            *interactable <<
                            if pressed {
                                Event3d::ButtonDown(key_code.into())
                            } else {
                                Event3d::ButtonUp(key_code.into())
                            };

                            *interactable << Event3d::Combos(self.combo_listener);
                        }
                    }
                }
                Event::ReceivedCharacter(c) => self.imgui.add_input_character(c),
                _ => {}
            }
        }

        for interactable in self.interactables.keys() {
            *interactable << Event3d::Frame
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

#[derive(Compact, Clone)]
pub struct AddInteractable2d(pub ID);

impl Recipient<AddInteractable2d> for UserInterface {
    fn receive(&mut self, msg: &AddInteractable2d) -> Fate {
        match *msg {
            AddInteractable2d(id) => {
                if !self.interactables_2d.contains(&id) {
                    self.interactables_2d.insert(0, id);
                }
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
pub struct RemoveInteractable2d(pub ID);

impl Recipient<RemoveInteractable2d> for UserInterface {
    fn receive(&mut self, msg: &RemoveInteractable2d) -> Fate {
        match *msg {
            RemoveInteractable2d(id) => {
                if let Some(idx) = self.interactables_2d.iter().position(|i| *i == id) {
                    self.interactables_2d.remove(idx);
                }
                Fate::Live
            }
        }
    }
}

#[derive(Copy, Clone)]
pub struct OnPanic;

impl Recipient<OnPanic> for UserInterface {
    fn receive(&mut self, _msg: &OnPanic) -> Fate {
        // so we don't wait forever for crashed actors to render UI
        self.interactables_2d
            .retain(|id| *id == camera_control::CameraControl::id());
        self.interactables_2d_todo
            .retain(|id| *id == camera_control::CameraControl::id());
        Fate::Live
    }
}

#[derive(Copy, Clone)]
pub struct Focus(pub ID);

impl Recipient<Focus> for UserInterface {
    fn receive(&mut self, msg: &Focus) -> Fate {
        match *msg {
            Focus(id) => {
                self.focused_interactables.insert(id);
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
    MouseMove(P2),
    MouseMove3d(P3),
    ButtonDown(combo::Button),
    ButtonUp(combo::Button),
    Combos(combo::ComboListener),
    Frame,
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
                        from: self.drag_start_3d
                            .expect("active interactable but no drag start"),
                        from2d: self.drag_start_2d
                            .expect("active interactable but no drag start"),
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

                for interactable in &self.focused_interactables {
                    *interactable << Event3d::MouseMove3d(self.cursor_3d);
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
        if self.parked_frame.is_some() {
            let target =
                std::mem::replace(&mut self.parked_frame, None).expect("Should have parked target");
            target.finish().unwrap();
        }

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
                self.parked_frame =
                    Some(unsafe { Box::from_raw(target_ptr as *mut ::monet::glium::Frame) });

                let size_points = self.window
                    .get_window()
                    .unwrap()
                    .get_inner_size_points()
                    .unwrap();
                let size_pixels = self.window
                    .get_window()
                    .unwrap()
                    .get_inner_size_pixels()
                    .unwrap();
                let ui = Box::new(self.imgui.frame(size_points, size_pixels, 1.0 / 60.0));

                self.imgui_capture_keyboard = ui.want_capture_keyboard();
                self.imgui_capture_mouse = ui.want_capture_mouse();

                let texts: Vec<_> = self.persistent_debug_text
                    .iter()
                    .chain(self.debug_text.iter())
                    .collect();

                ui.window(im_str!("Debug Info"))
                    .size((600.0, 200.0), ImGuiSetCond_FirstUseEver)
                    .collapsible(false)
                    .build(|| for (key, &(ref text, ref color)) in texts {
                               ui.text_colored(*color, im_str!("{}:\n{}", key, text));
                           });

                self.interactables_2d_todo = self.interactables_2d.clone();

                Self::id() << Ui2dDrawn { ui_ptr: Box::into_raw(ui) as usize };


                Fate::Live
            }
        }
    }
}

#[derive(Copy, Clone)]
pub struct DrawUI2d {
    pub ui_ptr: usize,
    pub return_to: ID,
}

#[derive(Copy, Clone)]
pub struct Ui2dDrawn {
    pub ui_ptr: usize,
}

impl Recipient<Ui2dDrawn> for UserInterface {
    fn receive(&mut self, msg: &Ui2dDrawn) -> Fate {
        match *msg {
            Ui2dDrawn { ui_ptr } => {
                let ui = unsafe { Box::from_raw(ui_ptr as *mut ::imgui::Ui) };

                if let Some(id) = self.interactables_2d_todo.pop() {
                    id <<
                    DrawUI2d {
                        ui_ptr: Box::into_raw(ui) as usize,
                        return_to: Self::id(),
                    }
                } else {
                    let mut target = std::mem::replace(&mut self.parked_frame, None)
                        .expect("Should have parked target");
                    self.imgui_renderer.render(&mut *target, *ui).unwrap();
                    target.finish().unwrap();
                }
            }
        }


        Fate::Live
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
            AddDebugText {
                ref key,
                ref text,
                ref color,
                persistent,
            } => {
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

pub fn setup(renderables: Vec<ID>, env: &'static environment::Environment, window: &GlutinFacade) {
    let mut renderer = Renderer::new(window.clone());
    let mut scene = Scene::new();
    scene.eye.position *= 30.0;
    scene.renderables = renderables;
    renderer.scenes.insert(0, scene);

    ::monet::setup(renderer);

    UserInterface::register_with_state(UserInterface::new(window.clone()));
    UserInterface::handle::<AddInteractable>();
    UserInterface::handle::<AddInteractable2d>();
    UserInterface::handle::<RemoveInteractable>();
    UserInterface::handle::<RemoveInteractable2d>();
    UserInterface::handle::<Focus>();
    UserInterface::handle_critically::<ProcessEvents>();
    UserInterface::handle_critically::<StartFrame>();
    UserInterface::handle_critically::<Projected3d>();
    UserInterface::handle_critically::<Submitted>();
    UserInterface::handle_critically::<Ui2dDrawn>();
    UserInterface::handle_critically::<OnPanic>();
    UserInterface::handle_critically::<AddDebugText>();

    camera_control::setup(env);
}
