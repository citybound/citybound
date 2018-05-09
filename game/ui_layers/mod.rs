use kay::{Actor, ActorSystem, World};
use stagemaster::{Interactable2d, Interactable2dID, UserInterface, UserInterfaceID};
use imgui::ImGuiSetCond_FirstUseEver;

#[repr(usize)]
pub enum UILayer {
    Base,
    Gesture,
    Info,
    Debug,
}

#[derive(Compact, Clone)]
pub struct LayerSelection {
    id: LayerSelectionID,
}

impl LayerSelection {
    pub fn spawn(
        id: LayerSelectionID,
        user_interface: UserInterfaceID,
        world: &mut World,
    ) -> LayerSelection {
        user_interface.add_2d(id.into(), world);
        LayerSelection { id }
    }
}

impl Interactable2d for LayerSelection {
    fn draw(&mut self, world: &mut World, ui: &::imgui::Ui<'static>) {
        ui.window(im_str!("UI Mode"))
            .size((200.0, 50.0), ImGuiSetCond_FirstUseEver)
            .collapsible(false)
            .build(|| {

                if ui.small_button(im_str!("Planning")) {
                    UserInterface::local_first(world).set_current_layer(
                        Some(
                            UILayer::Gesture as usize,
                        ),
                        world,
                    );
                }
                if ui.small_button(im_str!("Info")) {
                    UserInterface::local_first(world).set_current_layer(
                        Some(
                            UILayer::Info as usize,
                        ),
                        world,
                    );
                }
            });
    }
}

pub fn setup(system: &mut ActorSystem, user_interface: UserInterfaceID) {
    system.register::<LayerSelection>();
    self::kay_auto::auto_setup(system);
    LayerSelectionID::spawn(user_interface, &mut system.world());
}

mod kay_auto;
pub use self::kay_auto::*;
