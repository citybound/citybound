use kay::{Swarm, ToRandom, Recipient, ActorSystem, Individual, Fate};
use descartes::{Into2d, P3};
use core::geometry::AnyShape;
use core::ui::{UserInterface, VirtualKeyCode, KeyOrButton, KeyCombination, InterchangeableKeys, intersection};
use core::settings::Settings;
use super::{CurrentPlan};

#[derive(Clone, Default)]
pub struct LaneStrokeCanvas;


impl LaneStrokeCanvas {
    fn new() -> LaneStrokeCanvas {
        ()
    }
}

impl Individual for LaneStrokeCanvas{}

use core::ui::Event3d;
use super::{Commit, Undo, Redo, WithLatestNode, Materialize,
                         CreateGrid, DeleteSelection, SetSelectionMode,
                         SetNLanes, ToggleBothSides};

use core::ui::UIInput;

impl Recipient<UIInput> for LaneStrokeCanvas {
    fn receive(&mut self, msg: &UIInput) -> Fate {
        for event in msg.mouse_actions.into_iter(){
            match *event {
                Event3d::HoverStarted{at} | Event3d::HoverOngoing{at} => {
                    CurrentPlan::id() << WithLatestNode(at.into_2d(), true);
                },
                Event3d::DragStarted{at} => {
                    CurrentPlan::id() << WithLatestNode(at.into_2d(), true);
                    CurrentPlan::id() << Commit(true, at.into_2d());
                },
                Event3d::DragFinished{..} => {
                },
                _ => ()
            }
        }
        for name in msg.button_events.into_iter(){
            if name == "Undo"{
                CurrentPlan::id() << Undo;
            }
            if name == "Redo"{
                CurrentPlan::id() << Undo;
            }
            if name == "Spawn Cars"{
                Swarm::<::game::lanes_and_cars::Lane>::all() << ToRandom{
                    n_recipients: 5000,
                    message: Event3d::DragFinished{from: P3::new(0.0, 0.0, 0.0), to: P3::new(0.0, 0.0, 0.0)}
                };
            }
            if name == "Finalize"{
                CurrentPlan::id() << Materialize;
            }
            if name == "Create Grid"{
                CurrentPlan::id() << CreateGrid(10);
            }
            if name == "Create Big Grid"{
                CurrentPlan::id() << CreateGrid(15);
            }
            if name == "Delete Selection"{
                CurrentPlan::id() << DeleteSelection;
            }
            if name == "Set road to 1 lane"{CurrentPlan::id() << SetNLanes(1)}
            if name == "Set road to 2 lane"{CurrentPlan::id() << SetNLanes(2)}
            if name == "Set road to 3 lane"{CurrentPlan::id() << SetNLanes(3)}
            if name == "Set road to 4 lane"{CurrentPlan::id() << SetNLanes(4)}
            if name == "Set road to 5 lane"{CurrentPlan::id() << SetNLanes(5)}
            if name == "Set road to 6 lane"{CurrentPlan::id() << SetNLanes(6)}
            if name == "Set road to 7 lane"{CurrentPlan::id() << SetNLanes(7)}
            if name == "Set road to 8 lane"{CurrentPlan::id() << SetNLanes(8)}
            if name == "Set road to 9 lane"{CurrentPlan::id() << SetNLanes(9)}
            if name == "Set road to one way"{CurrentPlan::id() << ToggleBothSides}
        }
        Fate::Live
    }
}

use ::monet::EyeMoved;

impl Recipient<EyeMoved> for LaneStrokeCanvas{
    fn receive(&mut self, msg: &EyeMoved) -> Fate {match *msg {
        EyeMoved{eye, ..} => {
            if eye.position.z < 100.0 {
                CurrentPlan::id() << SetSelectionMode(false, false);
            } else if eye.position.z < 130.0 {
                CurrentPlan::id() << SetSelectionMode(true, false);
            } else {
                CurrentPlan::id() << SetSelectionMode(true, true);
            }
            Fate::Live
        }
    }}
}

use core::ui::AddInteractable;
use core::ui::Focus;

#[derive(Copy, Clone)]
struct AddToUI;

impl Recipient<AddToUI> for LaneStrokeCanvas {
    fn receive(&mut self, _msg: &AddToUI) -> Fate {
        UserInterface::id() << AddInteractable::Interactable3d(LaneStrokeCanvas::id(), AnyShape::Everywhere, 0);
        UserInterface::id() << Focus(LaneStrokeCanvas::id());
        ::monet::Renderer::id() << ::monet::AddEyeListener{scene_id: 0, listener: Self::id()};
        Fate::Live
    }
}

pub fn setup(system: &mut ActorSystem, settings: &mut Settings) {
    system.add_individual(LaneStrokeCanvas::new());
    system.add_inbox::<UIInput, LaneStrokeCanvas>();
    system.add_inbox::<EyeMoved, LaneStrokeCanvas>();

    settings.register_key(
        KeyCombination{
            keys: vec![
                InterchangeableKeys{
                    keys: vec![KeyOrButton::Key(VirtualKeyCode::Z)]
                },
                InterchangeableKeys{
                    keys: vec![KeyOrButton::Key(VirtualKeyCode::LControl),
                               KeyOrButton::Key(VirtualKeyCode::RControl),
                               KeyOrButton::Key(VirtualKeyCode::LWin),
                               KeyOrButton::Key(VirtualKeyCode::RWin)]
                },
            ],
        },
        "Undo"
    );

    settings.register_key(
        KeyCombination{
            keys: vec![
                InterchangeableKeys{
                    keys: vec![KeyOrButton::Key(VirtualKeyCode::Z)]
                },
                InterchangeableKeys{
                    keys: vec![KeyOrButton::Key(VirtualKeyCode::LControl),
                               KeyOrButton::Key(VirtualKeyCode::RControl),
                               KeyOrButton::Key(VirtualKeyCode::LWin),
                               KeyOrButton::Key(VirtualKeyCode::RWin)]
                },
                InterchangeableKeys{
                    keys: vec![KeyOrButton::Key(VirtualKeyCode::LShift),
                               KeyOrButton::Key(VirtualKeyCode::RShift)]
                },
            ],
        },
        "Redo"
    );

    settings.register_key(
        KeyCombination{
            keys: vec![
                InterchangeableKeys{
                    keys: vec![KeyOrButton::Key(VirtualKeyCode::Return)]
                },
            ],
        },
        "Finalize"
    );

    settings.register_key(
        KeyCombination{
            keys: vec![
                InterchangeableKeys{
                    keys: vec![KeyOrButton::Key(VirtualKeyCode::C)]
                },
            ],
        },
        "Car Spawning"
    );

    settings.register_key(
        KeyCombination{
            keys: vec![
                InterchangeableKeys{
                    keys: vec![KeyOrButton::Key(VirtualKeyCode::Delete),
                               KeyOrButton::Key(VirtualKeyCode::Back),
                               KeyOrButton::Key(VirtualKeyCode::Escape)]
                },
            ],
        },
        "Delete Selection"
    );

    settings.register_key(
        KeyCombination{
            keys: vec![
                InterchangeableKeys{
                    keys: vec![KeyOrButton::Key(VirtualKeyCode::G)]
                },
            ],
        },
        "Create Grid"
    );

    settings.register_key(
        KeyCombination{
            keys: vec![
                InterchangeableKeys{
                    keys: vec![KeyOrButton::Key(VirtualKeyCode::G)]
                },
                InterchangeableKeys{
                    keys: vec![KeyOrButton::Key(VirtualKeyCode::LShift),
                               KeyOrButton::Key(VirtualKeyCode::RShift)]
                },
            ],
        },
        "Create Big Grid" //TODO: How to make this not trigger the create grid as well
    );

    settings.register_key(
        KeyCombination{
            keys: vec![
                InterchangeableKeys{
                    keys: vec![KeyOrButton::Key(VirtualKeyCode::Key0)]
                },
            ],
        },
        "Set road to one way"
    );

    // There must be a better way!!!
    settings.register_key(
        KeyCombination{keys: vec![InterchangeableKeys{keys: vec![KeyOrButton::Key(VirtualKeyCode::Key1)]},],},
        "Set road to 1 lane"
    );
    settings.register_key(
        KeyCombination{keys: vec![InterchangeableKeys{keys: vec![KeyOrButton::Key(VirtualKeyCode::Key2)]},],},
        "Set road to 2 lane"
    );
    settings.register_key(
        KeyCombination{keys: vec![InterchangeableKeys{keys: vec![KeyOrButton::Key(VirtualKeyCode::Key3)]},],},
        "Set road to 3 lane"
    );
    settings.register_key(
        KeyCombination{keys: vec![InterchangeableKeys{keys: vec![KeyOrButton::Key(VirtualKeyCode::Key4)]},],},
        "Set road to 4 lane"
    );
    settings.register_key(
        KeyCombination{keys: vec![InterchangeableKeys{keys: vec![KeyOrButton::Key(VirtualKeyCode::Key5)]},],},
        "Set road to 5 lane"
    );
    settings.register_key(
        KeyCombination{keys: vec![InterchangeableKeys{keys: vec![KeyOrButton::Key(VirtualKeyCode::Key6)]},],},
        "Set road to 6 lane"
    );
    settings.register_key(
        KeyCombination{keys: vec![InterchangeableKeys{keys: vec![KeyOrButton::Key(VirtualKeyCode::Key7)]},],},
        "Set road to 7 lane"
    );
    settings.register_key(
        KeyCombination{keys: vec![InterchangeableKeys{keys: vec![KeyOrButton::Key(VirtualKeyCode::Key8)]},],},
        "Set road to 8 lane"
    );
    settings.register_key(
        KeyCombination{keys: vec![InterchangeableKeys{keys: vec![KeyOrButton::Key(VirtualKeyCode::Key9)]},],},
        "Set road to 9 lane"
    );


    LaneStrokeCanvas::id() << AddToUI;
}