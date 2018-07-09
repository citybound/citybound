use kay::{ActorSystem, World, External};
use compact::COption;
use monet::{RendererID, Movement};
use descartes::{P2, P3, V3};
use combo::Button::*;
use super::combo::{Bindings, Combo2};
use super::environment::Environment;

#[derive(Clone)]
pub struct CameraControlSettings {
    pub rotation_speed: f32,
    pub move_speed: f32,
    pub zoom_speed: f32,
    pub invert_y: bool,

    pub bindings: Bindings,
}

impl Default for CameraControlSettings {
    fn default() -> Self {
        CameraControlSettings {
            rotation_speed: 1.0f32,
            zoom_speed: 1.0f32,
            move_speed: 1.0f32,
            invert_y: false,
            bindings: Bindings::new(vec![
                ("Move Forward", Combo2::new(&[Up], &[W])),
                ("Move Backward", Combo2::new(&[Down], &[S])),
                ("Move Left", Combo2::new(&[Left], &[A])),
                ("Move Right", Combo2::new(&[Right], &[D])),
                ("Pan", Combo2::new(&[LShift], &[RShift])),
                ("Yaw", Combo2::new(&[LAlt], &[RightMouseButton])),
                ("Pitch", Combo2::new(&[LAlt], &[RightMouseButton])),
            ]),
        }
    }
}

#[derive(Compact, Clone)]
pub struct CameraControl {
    id: CameraControlID,
}

use user_interface::{Event3d, Interactable3d, Interactable3dID, UserInterfaceID};

impl CameraControl {
    pub fn spawn(
        id: CameraControlID,
        renderer_id: RendererID,
        ui_id: UserInterfaceID,
        env: Environment,
        world: &mut World,
    ) -> Self {
        CameraControl { id }
    }
}

impl Interactable3d for CameraControl {
    /// Critical
    fn on_event(&mut self, event: Event3d, world: &mut World) {}
}

use user_interface::{Interactable2d, Interactable2dID};

impl Interactable2d for CameraControl {
    /// Critical
    fn draw(&mut self, _: &mut World, ui: &()) {}
}

pub fn setup(system: &mut ActorSystem) {
    system.register::<CameraControl>();
    auto_setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;
