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
        // js! {
        //     window.cbReactApp.boundSetState(oldState => update(oldState, {
        //         households: {
        //             householdInfo: {
        //                 [@{Serde(id)}]: {"$set": {
        //                     core: @{Serde(core)}
        //                 }}
        //             }
        //         }
        //     }));
        // }

        // TODO: horrible workaround for Compact mis-serialising the best_offer COption (probably
        // because it's inside a CDict)
        let decision_state_workaround = match core.decision_state {
            ::economy::households::DecisionState::Choosing(m, i, ref rsrc, _) => {
                ::economy::households::DecisionState::Choosing(
                    m,
                    i,
                    rsrc.clone(),
                    ::compact::CDict::new(),
                )
            }
            ref other => other.clone(),
        };

        js! {
            window.cbReactApp.boundSetState(oldState => update(oldState, {
                households: {
                    householdInfo: {
                        [@{Serde(id)}]: {"$set": {
                            core: {
                                resources: @{Serde(&core.resources)},
                                member_resources: @{Serde(&core.member_resources)},
                                member_tasks: @{Serde(&core.member_tasks)},
                                decision_state: @{Serde(decision_state_workaround)},
                                used_offers: @{Serde(&core.used_offers)},
                                member_used_offers: @{Serde(&core.member_used_offers)},
                                provided_offers: @{Serde(&core.provided_offers)},
                            }
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
