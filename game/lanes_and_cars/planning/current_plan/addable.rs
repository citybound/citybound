use kay::{ID, ActorSystem, Fate};
use kay::swarm::{Swarm, SubActor};
use descartes::Band;
use stagemaster::geometry::{CPath, AnyShape};

use super::CurrentPlan;

#[derive(SubActor, Compact, Clone)]
pub struct Addable {
    _id: Option<ID>,
    path: CPath,
}

impl Addable {
    pub fn new(path: CPath) -> Self {
        Addable { _id: None, path: path }
    }
}

use super::InitInteractable;
use stagemaster::{UserInterface, AddInteractable};

pub fn setup(system: &mut ActorSystem) {
    system.add(
        Swarm::<Addable>::new(),
        Swarm::<Addable>::subactors(|mut each_addable| {
            let ui_id = each_addable.world().id::<UserInterface>();
            let cp_id = each_addable.world().id::<CurrentPlan>();

            each_addable.on_create_with(move |_: &InitInteractable, addable, world| {
                world.send(
                    ui_id,
                    AddInteractable(
                        addable.id(),
                        AnyShape::Band(Band::new(addable.path.clone(), 3.0)),
                        3,
                    ),
                );
                Fate::Live
            });

            each_addable.on(move |_: &ClearInteractable, addable, world| {
                world.send(ui_id, RemoveInteractable(addable.id()));
                Fate::Die
            });

            each_addable.on(move |event, _, world| {
                match *event {
                    Event3d::HoverStarted { .. } |
                    Event3d::HoverOngoing { .. } => {
                        world.send(
                            cp_id,
                            ChangeIntent(Intent::CreateNextLane, IntentProgress::Preview),
                        );
                    }
                    Event3d::HoverStopped => {
                        world.send(cp_id, ChangeIntent(Intent::None, IntentProgress::Preview));
                    }
                    Event3d::DragStarted { .. } => {
                        world.send(
                            cp_id,
                            ChangeIntent(Intent::CreateNextLane, IntentProgress::Immediate),
                        );
                    }
                    _ => {}
                };
                Fate::Live
            })
        }),
    );
}

use super::ClearInteractable;
use stagemaster::RemoveInteractable;

use stagemaster::Event3d;
use super::{ChangeIntent, Intent, IntentProgress};
