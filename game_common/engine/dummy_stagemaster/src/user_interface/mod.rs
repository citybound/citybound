use kay::{ActorSystem, External, World, Actor};
use compact::{CVec, CString, COption};
use descartes::{N, P2, V2, P3, Into2d, Area, PointContainer};
use monet::{RendererID, RenderableID, SceneDescription};

use std::collections::{HashMap, HashSet};
use std::collections::BTreeMap;

use camera_control::CameraControlID;
use environment::Environment;

#[derive(Copy, Clone)]
pub enum Event3d {
    DragStarted {
        at: P3,
        at2d: P2,
    },
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
    HoverStarted {
        at: P3,
        at2d: P2,
    },
    HoverOngoing {
        at: P3,
        at2d: P2,
    },
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
    fn draw_ui_2d(&mut self, imgui_ui: &(), return_to: UserInterfaceID, world: &mut World) {}

    fn draw(&mut self, _world: &mut World, _ui: &()) {
        unimplemented!()
    }
}

#[derive(Compact, Clone)]
pub struct UserInterface {
    id: UserInterfaceID,
}

pub type UserInterfaceLayer = usize;

impl UserInterface {
    pub fn spawn(
        id: UserInterfaceID,
        window: (),
        events_loop: (),
        renderer_id: RendererID,
        env: Environment,
        world: &mut World,
    ) -> UserInterface {
        UserInterface { id }
    }

    /// Critical
    pub fn process_events(&mut self, world: &mut World) {}

    pub fn add(
        &mut self,
        layer: UserInterfaceLayer,
        id: Interactable3dID,
        area: &COption<Area>,
        z_index: usize,
        world: &mut World,
    ) {
    }

    pub fn remove(&mut self, layer: UserInterfaceLayer, id: Interactable3dID, world: &mut World) {}

    pub fn focus(&mut self, id: Interactable3dID, _: &mut World) {}

    pub fn unfocus(&mut self, id: Interactable3dID, _: &mut World) {}

    pub fn set_current_layer(&mut self, layer: Option<UserInterfaceLayer>, _: &mut World) {}

    pub fn find_hovered_interactable(&mut self, world: &mut World) {}

    pub fn add_2d(&mut self, id: Interactable2dID, _: &mut World) {}

    pub fn remove_2d(&mut self, id: Interactable2dID, _: &mut World) {}

    /// Critical
    pub fn on_panic(&mut self, _: &mut World) {}

    /// Critical
    pub fn start_frame(&mut self, world: &mut World) {}
}

use monet::{ProjectionRequester, ProjectionRequesterID};

impl ProjectionRequester for UserInterface {
    fn projected_3d(&mut self, position_3d: P3, world: &mut World) {}
}

use monet::{TargetProvider, TargetProviderID};

#[cfg_attr(feature = "cargo-clippy", allow(useless_format))]
impl TargetProvider for UserInterface {
    /// Critical
    fn submitted(&mut self, target: (), world: &mut World) {}
}

impl UserInterface {
    /// Critical
    pub fn ui_drawn(&mut self, imgui_ui: &(), world: &mut World) {}

    /// Critical
    pub fn add_debug_text(
        &mut self,
        key: &CString,
        text: &CString,
        color: &[f32; 4],
        persistent: bool,
        _: &mut World,
    ) {
    }
}

pub fn setup(system: &mut ActorSystem) {
    ::monet::setup(system);
    system.register::<UserInterface>();
    auto_setup(system);
    super::camera_control::setup(system);
}

pub fn spawn(
    world: &mut World,
    renderables: CVec<RenderableID>,
    env: Environment,
    window_builder: (),
    clear_color: (f32, f32, f32, f32),
) -> (UserInterfaceID, RendererID) {
    let mut scene = SceneDescription::new(renderables);
    scene.eye.position *= 30.0;
    let renderer_id = RendererID::spawn(scene, clear_color, world);

    unsafe {
        super::debug::DEBUG_RENDERER = Some(renderer_id);
    }

    let ui_id = UserInterfaceID::spawn((), (), renderer_id, env, world);

    (ui_id, renderer_id)
}

mod kay_auto;
pub use self::kay_auto::*;
