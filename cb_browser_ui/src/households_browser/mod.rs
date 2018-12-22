use kay::{World, ActorSystem, TypedID};
use stdweb::serde::Serde;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use stdweb::js_export;
use SYSTEM;

#[cfg_attr(all(target_arch = "wasm32", target_os = "unknown"), js_export)]
pub fn get_household_info(household_id: Serde<::economy::households::HouseholdID>) {
    let system = unsafe { &mut *SYSTEM };
    let world = &mut system.world();
    household_id
        .0
        .get_ui_info(BrowserHouseholdUIID::local_first(world).into(), world);
}

#[derive(Compact, Clone)]
pub struct BrowserHouseholdUI {
    id: BrowserHouseholdUIID,
}

impl BrowserHouseholdUI {
    pub fn spawn(id: BrowserHouseholdUIID, _: &mut World) -> BrowserHouseholdUI {
        BrowserHouseholdUI { id }
    }
}

use economy::households::ui::{HouseholdUI, HouseholdUIID};

impl HouseholdUI for BrowserHouseholdUI {
    fn on_household_ui_info(
        &mut self,
        id: ::economy::households::HouseholdID,
        core: &::economy::households::HouseholdCore,
        _world: &mut World,
    ) {
        js! {
            window.cbReactApp.boundSetState(oldState => update(oldState, {
                households: {
                    householdInfo: {
                        [@{Serde(id)}]: {"$set": {
                            core: @{Serde(core)}
                        }}
                    }
                }
            }));
        }
    }
}

mod kay_auto;
pub use self::kay_auto::*;

pub fn setup(system: &mut ActorSystem) {
    system.register::<BrowserHouseholdUI>();
    auto_setup(system);
}

pub fn spawn(world: &mut World) {
    BrowserHouseholdUIID::spawn(world);
}
