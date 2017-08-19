#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]

extern crate compact;
#[macro_use]
extern crate compact_macros;
extern crate kay;
extern crate monet;
extern crate descartes;
#[macro_use]
extern crate imgui;
extern crate imgui_sys;
extern crate imgui_glium_renderer;
extern crate glium;

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate app_dirs;

pub mod geometry;
pub mod environment;
pub mod combo;
pub mod camera_control;

use kay::{ID, ActorSystem, Fate, External};
use descartes::{N, P2, V2, P3, Into2d, Shape};
use monet::{RendererID, ProjectionRequesterID, RenderableID, SceneDescription, Display};
use monet::glium::glutin::{ContextBuilder, Event, WindowBuilder, WindowEvent, MouseScrollDelta,
                           ElementState, MouseButton, KeyboardInput};
use monet::glium::glutin::EventsLoop;
pub use monet::glium::glutin::VirtualKeyCode;
use geometry::AnyShape;
use std::collections::{HashMap, HashSet};
use imgui::{ImGui, ImVec2, ImVec4, ImGuiSetCond_FirstUseEver, ImGuiKey};
use imgui_sys::{ImFontConfig, ImGuiCol, ImFontConfig_DefaultConstructor};
use imgui_glium_renderer::Renderer as ImguiRenderer;
use std::collections::BTreeMap;

pub struct UserInterface {
    events_loop: EventsLoop,
    interactables: HashMap<ID, (AnyShape, usize)>,
    ui_inner: UserInterfaceInner,
}

struct UserInterfaceInner {
    window: Display,
    mouse_button_state: [bool; 5],
    combo_listener: combo::ComboListener,
    cursor_2d: P2,
    cursor_3d: P3,
    drag_start_2d: Option<P2>,
    drag_start_3d: Option<P3>,
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

impl UserInterface {
    fn new(window: Display, events_loop: EventsLoop) -> Self {
        let mut imgui = ImGui::init();
        let default_font = im_str!("game/assets/ClearSans-Regular.ttf\0");

        unsafe {
            let atlas = (*imgui_sys::igGetIO()).fonts;
            let mut config: ImFontConfig = ::std::mem::zeroed();
            ImFontConfig_DefaultConstructor(&mut config);
            config.oversample_h = 2;
            config.oversample_v = 2;
            imgui_sys::ImFontAtlas_AddFontFromFileTTF(
                atlas,
                default_font.as_ptr(),
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
            events_loop: events_loop,
            interactables: HashMap::new(),
            ui_inner: UserInterfaceInner {
                window: window,
                mouse_button_state: [false; 5],
                combo_listener: combo::ComboListener::default(),
                cursor_2d: P2::new(0.0, 0.0),
                cursor_3d: P3::new(0.0, 0.0, 0.0),
                drag_start_2d: None,
                drag_start_3d: None,
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
            },
        }
    }
}

#[derive(Copy, Clone)]
pub struct ProcessEvents;

pub fn setup(
    system: &mut ActorSystem,
    renderables: Vec<RenderableID>,
    env: &'static environment::Environment,
    window: WindowBuilder,
) {
    ::monet::setup(system);

    let context = ContextBuilder::new().with_vsync(true);
    let events_loop = EventsLoop::new();
    let display = Display::new(window, context, &events_loop).unwrap();


    system.add(UserInterface::new(display.clone(), events_loop), move |mut the_ui| {
        let ui_id = the_ui.world().id::<UserInterface>();

        let mut scene = SceneDescription::new(renderables.clone().into());
        scene.eye.position *= 30.0;
        let renderer_id = RendererID::spawn(
            External::new(display.clone()),
            vec![scene].into(),
            &mut the_ui.world(),
        );

        use monet::MSG_ProjectionRequester_projected_3d;

        the_ui.on_critical(move |_: &ProcessEvents, ui, world| {
            let scale = ui.ui_inner.imgui.display_framebuffer_scale();

            let ui_inner = &mut ui.ui_inner;

            ui.events_loop.poll_events(|event| {
                match event {
                    Event::WindowEvent { event: WindowEvent::Closed, .. } => ::std::process::exit(
                        0,
                    ),

                    Event::WindowEvent { event: WindowEvent::MouseWheel { delta, .. }, .. } => {
                        let v = match delta {
                            MouseScrollDelta::LineDelta(x, y) => {
                                V2::new(x * 50.0 as N, y * 50.0 as N)
                            }
                            MouseScrollDelta::PixelDelta(x, y) => V2::new(x as N, y as N),
                        };

                        ui_inner.imgui.set_mouse_wheel(v.y / (scale.1 * 50.0));

                        if !ui_inner.imgui_capture_mouse {
                            for interactable in &ui_inner.focused_interactables {
                                world.send(*interactable, Event3d::Scroll(v))
                            }
                        }
                    }
                    Event::WindowEvent {
                        event: WindowEvent::MouseMoved { position: (x, y), .. }, ..
                    } => {
                        ui_inner.cursor_2d = P2::new(x as N, y as N);

                        ui_inner.imgui.set_mouse_pos(
                            ui_inner.cursor_2d.x / scale.0,
                            ui_inner.cursor_2d.y / scale.1,
                        );

                        for interactable in &ui_inner.focused_interactables {
                            world.send(*interactable, Event3d::MouseMove(ui_inner.cursor_2d));
                        }

                        renderer_id.project_2d_to_3d(
                            0,
                            ui_inner.cursor_2d,
                            ProjectionRequesterID { _raw_id: ui_id },
                            world,
                        );
                    }
                    Event::WindowEvent {
                        event: WindowEvent::MouseInput { state: button_state, button, .. }, ..
                    } => {
                        let button_idx = match button {
                            MouseButton::Left => 0,
                            MouseButton::Right => 1,
                            MouseButton::Middle => 2,
                            _ => 4,
                        };
                        let pressed = button_state == ElementState::Pressed;
                        ui_inner.mouse_button_state[button_idx] = pressed;

                        ui_inner.imgui.set_mouse_down(&ui_inner.mouse_button_state);

                        if !ui_inner.imgui_capture_mouse {
                            ui_inner.combo_listener.update(&event);

                            if pressed {
                                ui_inner.drag_start_2d = Some(ui_inner.cursor_2d);
                                ui_inner.drag_start_3d = Some(ui_inner.cursor_3d);
                                // TODO: does this break something?
                                //let cursor_3d = ui.cursor_3d;
                                //ui.receive(&Projected3d { position_3d: cursor_3d });
                                ui_inner.active_interactable = ui_inner.hovered_interactable;
                                if let Some(active_interactable) = ui_inner.active_interactable {
                                    world.send(
                                        active_interactable,
                                        Event3d::DragStarted {
                                            at: ui_inner.cursor_3d,
                                            at2d: ui_inner.cursor_2d,
                                        },
                                    );
                                }
                            } else {
                                if let Some(active_interactable) = ui_inner.active_interactable {
                                    world.send(
                                        active_interactable,
                                        Event3d::DragFinished {
                                            from: ui_inner.drag_start_3d.expect(
                                                "active interactable but no drag start",
                                            ),
                                            from2d: ui_inner.drag_start_2d.expect(
                                                "active interactable but no drag start",
                                            ),
                                            to: ui_inner.cursor_3d,
                                            to2d: ui_inner.cursor_2d,
                                        },
                                    );
                                }
                                ui_inner.drag_start_2d = None;
                                ui_inner.drag_start_3d = None;
                                ui_inner.active_interactable = None;
                            }

                            for interactable in &ui_inner.focused_interactables {
                                world.send(
                                    *interactable,
                                    if pressed {
                                        Event3d::ButtonDown(button.into())
                                    } else {
                                        Event3d::ButtonUp(button.into())
                                    },
                                );

                                world.send(*interactable, Event3d::Combos(ui_inner.combo_listener));
                            }
                        }
                    }
                    Event::WindowEvent {
                        event: WindowEvent::KeyboardInput {
                            input: KeyboardInput { state, virtual_keycode: Some(key_code), .. }, ..
                        },
                        ..
                    } => {
                        let pressed = state == ElementState::Pressed;

                        if ui_inner.imgui_capture_keyboard {
                            match key_code {
                                VirtualKeyCode::Tab => ui_inner.imgui.set_key(0, pressed),
                                VirtualKeyCode::Left => ui_inner.imgui.set_key(1, pressed),
                                VirtualKeyCode::Right => ui_inner.imgui.set_key(2, pressed),
                                VirtualKeyCode::Up => ui_inner.imgui.set_key(3, pressed),
                                VirtualKeyCode::Down => ui_inner.imgui.set_key(4, pressed),
                                VirtualKeyCode::PageUp => ui_inner.imgui.set_key(5, pressed),
                                VirtualKeyCode::PageDown => ui_inner.imgui.set_key(6, pressed),
                                VirtualKeyCode::Home => ui_inner.imgui.set_key(7, pressed),
                                VirtualKeyCode::End => ui_inner.imgui.set_key(8, pressed),
                                VirtualKeyCode::Delete => ui_inner.imgui.set_key(9, pressed),
                                VirtualKeyCode::Back => ui_inner.imgui.set_key(10, pressed),
                                VirtualKeyCode::Return => ui_inner.imgui.set_key(11, pressed),
                                VirtualKeyCode::Escape => ui_inner.imgui.set_key(12, pressed),
                                VirtualKeyCode::A => ui_inner.imgui.set_key(13, pressed),
                                VirtualKeyCode::C => ui_inner.imgui.set_key(14, pressed),
                                VirtualKeyCode::V => ui_inner.imgui.set_key(15, pressed),
                                VirtualKeyCode::X => ui_inner.imgui.set_key(16, pressed),
                                VirtualKeyCode::Y => ui_inner.imgui.set_key(17, pressed),
                                VirtualKeyCode::Z => ui_inner.imgui.set_key(18, pressed),
                                VirtualKeyCode::LControl | VirtualKeyCode::RControl => {
                                    ui_inner.imgui.set_key_ctrl(pressed)
                                }
                                VirtualKeyCode::LShift | VirtualKeyCode::RShift => {
                                    ui_inner.imgui.set_key_shift(pressed)
                                }
                                VirtualKeyCode::LAlt | VirtualKeyCode::RAlt => {
                                    ui_inner.imgui.set_key_alt(pressed)
                                }
                                VirtualKeyCode::LWin | VirtualKeyCode::RWin => {
                                    ui_inner.imgui.set_key_super(pressed)
                                }
                                _ => {}
                            }
                        } else {
                            ui_inner.combo_listener.update(&event);

                            for interactable in &ui_inner.focused_interactables {
                                world.send(
                                    *interactable,
                                    if pressed {
                                        Event3d::ButtonDown(key_code.into())
                                    } else {
                                        Event3d::ButtonUp(key_code.into())
                                    },
                                );

                                world.send(*interactable, Event3d::Combos(ui_inner.combo_listener));
                            }
                        }
                    }
                    Event::WindowEvent { event: WindowEvent::ReceivedCharacter(c), .. } => {
                        ui_inner.imgui.add_input_character(c)
                    }
                    _ => {}
                }


            });

            for interactable in ui.interactables.keys() {
                world.send(*interactable, Event3d::Frame)
            }

            Fate::Live
        });

        the_ui.on(|&AddInteractable(id, ref shape, z_index), ui, _| {
            ui.interactables.insert(id, (shape.clone(), z_index));
            Fate::Live
        });

        the_ui.on(|&AddInteractable2d(id), ui, _| {
            if !ui.ui_inner.interactables_2d.contains(&id) {
                ui.ui_inner.interactables_2d.insert(0, id);
            }
            Fate::Live
        });

        the_ui.on(|&RemoveInteractable(id), ui, _| {
            ui.interactables.remove(&id);
            Fate::Live
        });

        the_ui.on(|&RemoveInteractable2d(id), ui, _| {
            if let Some(idx) = ui.ui_inner.interactables_2d.iter().position(|i| *i == id) {
                ui.ui_inner.interactables_2d.remove(idx);
            }
            Fate::Live
        });

        the_ui.on(|&Focus(id), ui, _| {
            ui.ui_inner.focused_interactables.insert(id);
            Fate::Live
        });

        let cc_id = the_ui.world().id::<camera_control::CameraControl>();

        the_ui.on_critical(move |_: &OnPanic, ui, _| {
            // so we don't wait forever for crashed actors to render UI
            ui.ui_inner.interactables_2d.retain(|id| *id == cc_id);
            ui.ui_inner.interactables_2d_todo.retain(|id| *id == cc_id);
            Fate::Live
        });

        the_ui.on_critical(|&MSG_ProjectionRequester_projected_3d(position_3d),
         ui,
         world| {
            ui.ui_inner.cursor_3d = position_3d;
            if let Some(active_interactable) = ui.ui_inner.active_interactable {
                world.send(
                    active_interactable,
                    Event3d::DragOngoing {
                        from: ui.ui_inner.drag_start_3d.expect(
                            "active interactable but no drag start",
                        ),
                        from2d: ui.ui_inner.drag_start_2d.expect(
                            "active interactable but no drag start",
                        ),
                        to: position_3d,
                        to2d: ui.ui_inner.cursor_2d,
                    },
                );
            } else {
                let new_hovered_interactable = ui.interactables
                    .iter()
                    .filter(|&(_id, &(ref shape, _z_index))| {
                        shape.contains(position_3d.into_2d())
                    })
                    .max_by_key(|&(_id, &(ref _shape, z_index))| z_index)
                    .map(|(id, _shape)| *id);

                if ui.ui_inner.hovered_interactable != new_hovered_interactable {
                    if let Some(previous) = ui.ui_inner.hovered_interactable {
                        world.send(previous, Event3d::HoverStopped);
                    }
                    if let Some(next) = new_hovered_interactable {
                        world.send(
                            next,
                            Event3d::HoverStarted {
                                at: ui.ui_inner.cursor_3d,
                                at2d: ui.ui_inner.cursor_2d,
                            },
                        );
                    }
                } else if let Some(hovered_interactable) = ui.ui_inner.hovered_interactable {
                    world.send(
                        hovered_interactable,
                        Event3d::HoverOngoing {
                            at: ui.ui_inner.cursor_3d,
                            at2d: ui.ui_inner.cursor_2d,
                        },
                    );
                }
                ui.ui_inner.hovered_interactable = new_hovered_interactable;
            }

            for interactable in &ui.ui_inner.focused_interactables {
                world.send(*interactable, Event3d::MouseMove3d(ui.ui_inner.cursor_3d));
            }
            Fate::Live
        });

        the_ui.on_critical(move |_: &StartFrame, ui, world| {
            if ui.ui_inner.parked_frame.is_some() {
                let target = std::mem::replace(&mut ui.ui_inner.parked_frame, None)
                    .expect("Should have parked target");
                target.finish().unwrap();
            }

            let target = External::new(ui.ui_inner.window.draw());

            renderer_id.submit(target, monet::TargetProviderID { _raw_id: ui_id }, world);

            Fate::Live
        });

        use monet::MSG_TargetProvider_submitted;

        the_ui.on_critical(move |&MSG_TargetProvider_submitted(ref given_target),
              ui,
              world| {
            ui.ui_inner.parked_frame = Some(given_target.steal().into_box());

            let size_points = ui.ui_inner
                .window
                .gl_window()
                .get_inner_size_points()
                .unwrap();
            let size_pixels = ui.ui_inner
                .window
                .gl_window()
                .get_inner_size_pixels()
                .unwrap();

            // somewhat of a hack to override the local lifetime of the returned imgui::Ui
            let imgui_ui_shortlived = ui.ui_inner.imgui.frame(
                size_points,
                size_pixels,
                1.0 / 60.0,
            );
            let imgui_ui = unsafe {
                Box::new(std::mem::transmute::<_, ::imgui::Ui<'static>>(
                    imgui_ui_shortlived,
                ))
            };

            ui.ui_inner.imgui_capture_keyboard = imgui_ui.want_capture_keyboard();
            ui.ui_inner.imgui_capture_mouse = imgui_ui.want_capture_mouse();

            let texts: Vec<_> = ui.ui_inner
                .persistent_debug_text
                .iter()
                .chain(ui.ui_inner.debug_text.iter())
                .collect();

            imgui_ui
                .window(im_str!("Debug Info"))
                .size((600.0, 200.0), ImGuiSetCond_FirstUseEver)
                .collapsible(false)
                .build(|| for (key, &(ref text, ref color)) in texts {
                    imgui_ui.text_colored(*color, im_str!("{}:\n{}", key, text));
                });

            ui.ui_inner.interactables_2d_todo = ui.ui_inner.interactables_2d.clone();

            world.send(ui_id, Ui2dDrawn { imgui_ui: External::from_box(imgui_ui) });

            Fate::Live
        });

        the_ui.on_critical(move |&Ui2dDrawn { ref imgui_ui }, ui, world| {
            if let Some(id) = ui.ui_inner.interactables_2d_todo.pop() {
                world.send(
                    id,
                    DrawUI2d {
                        imgui_ui: imgui_ui.steal(),
                        return_to: ui_id,
                    },
                );
            } else {
                let mut target = std::mem::replace(&mut ui.ui_inner.parked_frame, None)
                    .expect("Should have parked target");
                ui.ui_inner
                    .imgui_renderer
                    .render(&mut *target, unsafe {
                        ::std::ptr::read(Box::into_raw(imgui_ui.steal().into_box()))
                    })
                    .unwrap();
                target.finish().unwrap();
            }

            Fate::Live
        });

        the_ui.on_critical(|&AddDebugText {
             ref key,
             ref text,
             ref color,
             persistent,
         },
         ui,
         _| {
            let target = if persistent {
                &mut ui.ui_inner.persistent_debug_text
            } else {
                &mut ui.ui_inner.debug_text
            };
            target.insert(key.iter().cloned().collect(), (
                text.iter().cloned().collect(),
                *color,
            ));
            Fate::Live
        });
    });

    camera_control::setup(system, env);
}

#[derive(Compact, Clone)]
pub struct AddInteractable(pub ID, pub AnyShape, pub usize);


#[derive(Compact, Clone)]
pub struct AddInteractable2d(pub ID);

#[derive(Copy, Clone)]
pub struct RemoveInteractable(pub ID);

#[derive(Copy, Clone)]
pub struct RemoveInteractable2d(pub ID);

#[derive(Copy, Clone)]
pub struct Focus(pub ID);

#[derive(Copy, Clone)]
pub struct OnPanic;

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

#[derive(Copy, Clone)]
pub struct StartFrame;

#[derive(Compact, Clone)]
pub struct DrawUI2d {
    pub imgui_ui: External<::imgui::Ui<'static>>,
    pub return_to: ID,
}

#[derive(Compact, Clone)]
pub struct Ui2dDrawn {
    pub imgui_ui: External<::imgui::Ui<'static>>,
}

use compact::CVec;

#[derive(Compact, Clone)]
pub struct AddDebugText {
    pub key: CVec<char>,
    pub text: CVec<char>,
    pub color: [f32; 4],
    pub persistent: bool,
}
