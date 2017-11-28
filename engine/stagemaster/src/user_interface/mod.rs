use kay::{ActorSystem, External, World, Actor};
use compact::{CVec, CString};
use descartes::{N, P2, V2, P3, Into2d, Shape};
use monet::{RendererID, RenderableID, SceneDescription, Display};
use monet::glium::glutin::{ContextBuilder, Event, WindowBuilder, WindowEvent, MouseScrollDelta,
                           ElementState, MouseButton, KeyboardInput};
use monet::glium::glutin::EventsLoop;
pub use monet::glium::glutin::VirtualKeyCode;
use std::collections::{HashMap, HashSet};
use imgui::{ImGui, ImVec2, ImVec4, ImGuiSetCond_FirstUseEver, ImGuiKey};
use imgui_sys::{ImFontConfig, ImGuiCol, ImFontConfig_DefaultConstructor};
use imgui_glium_renderer::Renderer as ImguiRenderer;
use std::collections::BTreeMap;

use geometry::AnyShape;
use camera_control::CameraControlID;
use environment::Environment;

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
    ButtonDown(::combo::Button),
    ButtonUp(::combo::Button),
    Combos(::combo::ComboListener),
    Frame,
}

pub trait Interactable3d {
    fn on_event(&mut self, event: Event3d, world: &mut World);
}

pub trait Interactable2d {
    fn draw_ui_2d(
        &mut self,
        imgui_ui: &External<::imgui::Ui<'static>>,
        return_to: UserInterfaceID,
        world: &mut World,
    );
}

#[derive(Compact, Clone)]
pub struct UserInterface {
    id: UserInterfaceID,
    inner: External<UserInterfaceInner>,
}

pub struct UserInterfaceInner {
    events_loop: EventsLoop,
    window: Display,
    renderer_id: RendererID,
    camera_control_id: CameraControlID,
    mouse_button_state: [bool; 5],
    combo_listener: ::combo::ComboListener,
    cursor_2d: P2,
    cursor_3d: P3,
    drag_start_2d: Option<P2>,
    drag_start_3d: Option<P3>,
    interactables: HashMap<Interactable3dID, (AnyShape, usize)>,
    hovered_interactable: Option<Interactable3dID>,
    active_interactable: Option<Interactable3dID>,
    focused_interactables: HashSet<Interactable3dID>,
    interactables_2d: Vec<Interactable2dID>,
    interactables_2d_todo: Vec<Interactable2dID>,
    parked_frame: Option<Box<::monet::glium::Frame>>,
    imgui: ImGui,
    imgui_capture_keyboard: bool,
    imgui_capture_mouse: bool,
    imgui_renderer: ImguiRenderer,
    debug_text: BTreeMap<String, (String, [f32; 4])>,
    persistent_debug_text: BTreeMap<String, (String, [f32; 4])>,
    panicked: bool,
}

impl ::std::ops::Deref for UserInterface {
    type Target = UserInterfaceInner;

    fn deref(&self) -> &UserInterfaceInner {
        &self.inner
    }
}

impl ::std::ops::DerefMut for UserInterface {
    fn deref_mut(&mut self) -> &mut UserInterfaceInner {
        &mut self.inner
    }
}

impl UserInterface {
    pub fn spawn(
        id: UserInterfaceID,
        window: &External<Display>,
        events_loop: &External<EventsLoop>,
        renderer_id: RendererID,
        env: Environment,
        world: &mut World,
    ) -> UserInterface {
        let mut imgui = ImGui::init();
        let default_font = include_bytes!("../../../../game/assets/ClearSans-Regular.ttf");

        unsafe {
            let atlas = (*::imgui_sys::igGetIO()).fonts;
            let mut config: ImFontConfig = ::std::mem::zeroed();
            ImFontConfig_DefaultConstructor(&mut config);
            config.oversample_h = 2;
            config.oversample_v = 2;
            config.font_data_owned_by_atlas = false;
            ::imgui_sys::ImFontAtlas_AddFontFromMemoryTTF(
                atlas,
                ::std::mem::transmute(default_font.as_ptr()),
                default_font.len() as i32,
                16.0,
                &config,
                ::std::ptr::null(),
            );

            let style = imgui.style_mut();
            style.window_rounding = 4.0;
            style.grab_rounding = 3.0;
            style.scrollbar_rounding = 3.0;
            style.frame_rounding = 3.0;
            style.scrollbar_size = 14.0;
            style.window_title_align = ImVec2::new(0.5, 0.5);
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

        let imgui_renderer = ImguiRenderer::init(&mut imgui, &**window).unwrap();

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
            id,
            inner: External::new(UserInterfaceInner {
                window: *window.steal().into_box(),
                events_loop: *events_loop.steal().into_box(),
                renderer_id: renderer_id,
                camera_control_id: CameraControlID::spawn(renderer_id, id, env, world),
                mouse_button_state: [false; 5],
                combo_listener: ::combo::ComboListener::default(),
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
                panicked: false,
            }),
        }
    }

    fn poll_events(&mut self) -> Vec<Event> {
        let mut res = Vec::new();
        self.events_loop.poll_events(|event| { res.push(event); });
        res
    }

    /// Critical
    pub fn process_events(&mut self, world: &mut World) {
        let scale = self.imgui.display_framebuffer_scale();
        let events = self.poll_events();

        for event in events {
            if let Event::WindowEvent { event: ref window_event, .. } = event {
                match *window_event {
                    WindowEvent::Closed => world.shutdown(),

                    WindowEvent::MouseWheel { delta, .. } => {
                        let v = match delta {
                            MouseScrollDelta::LineDelta(x, y) => {
                                V2::new(x * 50.0 as N, y * 50.0 as N)
                            }
                            MouseScrollDelta::PixelDelta(x, y) => V2::new(x as N, y as N),
                        };

                        self.imgui.set_mouse_wheel(v.y / (scale.1 * 50.0));

                        if !self.imgui_capture_mouse {
                            for interactable in &self.focused_interactables {
                                interactable.on_event(Event3d::Scroll(v), world);
                            }
                        }
                    }
                    WindowEvent::MouseMoved { position: (x, y), .. } => {
                        self.cursor_2d = P2::new(x as N, y as N);

                        let mouse_x = self.cursor_2d.x / scale.0;
                        let mouse_y = self.cursor_2d.y / scale.1;
                        self.imgui.set_mouse_pos(mouse_x, mouse_y);

                        for interactable in &self.focused_interactables {
                            interactable.on_event(Event3d::MouseMove(self.cursor_2d), world);
                        }

                        self.renderer_id.project_2d_to_3d(
                            0,
                            self.cursor_2d,
                            self.id_as(),
                            world,
                        );
                    }
                    WindowEvent::MouseInput { state: button_state, button, .. } => {
                        let button_idx = match button {
                            MouseButton::Left => 0,
                            MouseButton::Right => 1,
                            MouseButton::Middle => 2,
                            _ => 4,
                        };
                        let pressed = button_state == ElementState::Pressed;
                        self.mouse_button_state[button_idx] = pressed;

                        let mouse_button_state = self.mouse_button_state;
                        self.imgui.set_mouse_down(&mouse_button_state);

                        if !self.imgui_capture_mouse {
                            self.combo_listener.update(&event);

                            if pressed {
                                self.drag_start_2d = Some(self.cursor_2d);
                                self.drag_start_3d = Some(self.cursor_3d);
                                // TODO: does this break something?
                                //let cursor_3d = self.cursor_3d;
                                //self.receive(&Projected3d { position_3d: cursor_3d });
                                self.active_interactable = self.hovered_interactable;
                                if let Some(active_interactable) = self.active_interactable {
                                    active_interactable.on_event(
                                        Event3d::DragStarted {
                                            at: self.cursor_3d,
                                            at2d: self.cursor_2d,
                                        },
                                        world,
                                    );
                                }
                            } else {
                                if let Some(active_interactable) = self.active_interactable {
                                    active_interactable.on_event(
                                        Event3d::DragFinished {
                                            from: self.drag_start_3d.expect(
                                                "active interactable but no drag start",
                                            ),
                                            from2d: self.drag_start_2d.expect(
                                                "active interactable but no drag start",
                                            ),
                                            to: self.cursor_3d,
                                            to2d: self.cursor_2d,
                                        },
                                        world,
                                    );
                                }
                                self.drag_start_2d = None;
                                self.drag_start_3d = None;
                                self.active_interactable = None;
                            }

                            for interactable in &self.focused_interactables {
                                interactable.on_event(
                                    if pressed {
                                        Event3d::ButtonDown(button.into())
                                    } else {
                                        Event3d::ButtonUp(button.into())
                                    },
                                    world,
                                );

                                interactable.on_event(Event3d::Combos(self.combo_listener), world);
                            }
                        }
                    }
                    WindowEvent::KeyboardInput {
                        input: KeyboardInput { state, virtual_keycode: Some(key_code), .. }, ..
                    } => {
                        let pressed = state == ElementState::Pressed;

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
                                interactable.on_event(
                                    if pressed {
                                        Event3d::ButtonDown(key_code.into())
                                    } else {
                                        Event3d::ButtonUp(key_code.into())
                                    },
                                    world,
                                );

                                interactable.on_event(Event3d::Combos(self.combo_listener), world);
                            }
                        }
                    }
                    WindowEvent::ReceivedCharacter(c) => self.imgui.add_input_character(c),
                    _ => {}
                }
            }
        }

        for interactable in self.interactables.keys() {
            interactable.on_event(Event3d::Frame, world)
        }
    }

    pub fn add(&mut self, id: Interactable3dID, shape: &AnyShape, z_index: usize, _: &mut World) {
        self.interactables.insert(id, (shape.clone(), z_index));
    }

    pub fn remove(&mut self, id: Interactable3dID, _: &mut World) {
        self.interactables.remove(&id);
    }

    pub fn focus(&mut self, id: Interactable3dID, _: &mut World) {
        self.focused_interactables.insert(id);
    }

    pub fn add_2d(&mut self, id: Interactable2dID, _: &mut World) {
        if !self.interactables_2d.contains(&id) {
            self.interactables_2d.insert(0, id);
        }
    }

    pub fn remove_2d(&mut self, id: Interactable2dID, _: &mut World) {
        if let Some(idx) = self.interactables_2d.iter().position(|i| *i == id) {
            self.interactables_2d.remove(idx);
        }
    }

    /// Critical
    pub fn on_panic(&mut self, _: &mut World) {
        // so we don't wait forever for crashed actors to render UI
        let cc_id = self.camera_control_id.into();
        self.interactables_2d.retain(|id| *id == cc_id);
        self.interactables_2d_todo.retain(|id| *id == cc_id);
        self.panicked = true;
    }

    /// Critical
    pub fn start_frame(&mut self, world: &mut World) {
        if self.parked_frame.is_some() {
            let target = ::std::mem::replace(&mut self.parked_frame, None).expect(
                "Should have parked target",
            );
            target.finish().unwrap();
        }

        let target = External::new(self.window.draw());

        self.renderer_id.submit(target, self.id_as(), world);
    }
}

use monet::{ProjectionRequester, ProjectionRequesterID};

impl ProjectionRequester for UserInterface {
    fn projected_3d(&mut self, position_3d: P3, world: &mut World) {
        self.cursor_3d = position_3d;
        if let Some(active_interactable) = self.active_interactable {
            active_interactable.on_event(
                Event3d::DragOngoing {
                    from: self.drag_start_3d.expect(
                        "active interactable but no drag start",
                    ),
                    from2d: self.drag_start_2d.expect(
                        "active interactable but no drag start",
                    ),
                    to: position_3d,
                    to2d: self.cursor_2d,
                },
                world,
            );
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
                    previous.on_event(Event3d::HoverStopped, world);
                }
                if let Some(next) = new_hovered_interactable {
                    next.on_event(
                        Event3d::HoverStarted { at: self.cursor_3d, at2d: self.cursor_2d },
                        world,
                    );
                }
            } else if let Some(hovered_interactable) = self.hovered_interactable {
                hovered_interactable.on_event(
                    Event3d::HoverOngoing {
                        at: self.cursor_3d,
                        at2d: self.cursor_2d,
                    },
                    world,
                );
            }
            self.hovered_interactable = new_hovered_interactable;
        }

        for interactable in &self.focused_interactables {
            interactable.on_event(Event3d::MouseMove3d(self.cursor_3d), world);
        }
    }
}

use monet::{TargetProvider, TargetProviderID};
use monet::glium::Frame;

#[allow(useless_format)]
impl TargetProvider for UserInterface {
    /// Critical
    fn submitted(&mut self, target: &External<Frame>, world: &mut World) {
        self.parked_frame = Some(target.steal().into_box());

        let size_points = self.window.gl_window().get_inner_size_points().unwrap();
        let size_pixels = self.window.gl_window().get_inner_size_pixels().unwrap();

        let imgui_ui = {
            // somewhat of a hack to override the local lifetime of the returned imgui::Ui
            let imgui_ui_shortlived = self.imgui.frame(size_points, size_pixels, 1.0 / 60.0);
            unsafe {
                Box::new(::std::mem::transmute::<_, ::imgui::Ui<'static>>(
                    imgui_ui_shortlived,
                ))
            }
        };

        self.imgui_capture_keyboard = imgui_ui.want_capture_keyboard();
        self.imgui_capture_mouse = imgui_ui.want_capture_mouse();

        let texts: Vec<_> = self.persistent_debug_text
            .clone()
            .into_iter()
            .chain(self.debug_text.clone().into_iter())
            .collect();

        imgui_ui
            .window(im_str!("Debug Info"))
            .size((230.0, 200.0), ImGuiSetCond_FirstUseEver)
            .collapsible(!self.panicked)
            .position((10.0, 10.0), ImGuiSetCond_FirstUseEver)
            .build(|| for (ref key, (ref text, ref color)) in texts {
                if text.lines().count() > 3 {
                    imgui_ui.tree_node(im_str!("{}", key)).build(|| {
                        imgui_ui.text_colored(*color, im_str!("{}", text));
                    });
                } else {
                    imgui_ui.text_colored(*color, im_str!("{}\n{}", key, text));
                }
            });

        self.interactables_2d_todo = self.interactables_2d.clone();
        self.id.ui_drawn(External::from_box(imgui_ui), world);
    }
}

impl UserInterface {
    /// Critical
    pub fn ui_drawn(&mut self, imgui_ui: &External<::imgui::Ui<'static>>, world: &mut World) {
        if let Some(interactable_2d) = self.interactables_2d_todo.pop() {
            interactable_2d.draw_ui_2d(imgui_ui.steal(), self.id, world);
        } else {
            let mut target = ::std::mem::replace(&mut self.parked_frame, None).expect(
                "Should have parked target",
            );
            self.imgui_renderer
                .render(&mut *target, unsafe {
                    ::std::ptr::read(Box::into_raw(imgui_ui.steal().into_box()))
                })
                .unwrap();
            target.finish().unwrap();
        }
    }

    /// Critical
    pub fn add_debug_text(
        &mut self,
        key: &CString,
        text: &CString,
        color: &[f32; 4],
        persistent: bool,
        _: &mut World,
    ) {
        let target = if persistent {
            &mut self.persistent_debug_text
        } else {
            &mut self.debug_text
        };
        target.insert(key.to_string(), (text.to_string(), *color));
    }
}

pub fn setup(
    system: &mut ActorSystem,
    renderables: CVec<RenderableID>,
    env: Environment,
    window_builder: WindowBuilder,
    clear_color: (f32, f32, f32, f32),
) -> (UserInterfaceID, RendererID) {
    ::monet::setup(system);
    system.register::<UserInterface>();
    auto_setup(system);

    super::camera_control::setup(system);

    let context = ContextBuilder::new().with_vsync(true);
    let events_loop = EventsLoop::new();
    let window = Display::new(window_builder, context, &events_loop).unwrap();

    let mut scene = SceneDescription::new(renderables);
    scene.eye.position *= 30.0;
    let renderer_id = RendererID::spawn(
        External::new(window.clone()),
        vec![scene].into(),
        clear_color,
        &mut system.world(),
    );

    unsafe {
        super::geometry::DEBUG_RENDERER = Some(renderer_id);
    }

    let ui_id = UserInterfaceID::spawn(
        External::new(window),
        External::new(events_loop),
        renderer_id,
        env,
        &mut system.world(),
    );

    (ui_id, renderer_id)
}

mod kay_auto;
pub use self::kay_auto::*;
