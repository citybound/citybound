use kay::{ID, ActorSystem, Fate};
use kay::swarm::{Swarm, SubActor};
use descartes::{N, Band, Into2d, Norm};
use stagemaster::geometry::{CPath, AnyShape};

use super::{SelectableStrokeRef, CurrentPlan};

#[derive(SubActor, Compact, Clone)]
pub struct Draggable {
    _id: Option<ID>,
    stroke_ref: SelectableStrokeRef,
    path: CPath,
}

impl Draggable {
    pub fn new(stroke_ref: SelectableStrokeRef, path: CPath) -> Self {
        Draggable {
            _id: None,
            stroke_ref: stroke_ref,
            path: path,
        }
    }
}

use super::InitInteractable;
use stagemaster::{UserInterface, AddInteractable};

pub fn setup(system: &mut ActorSystem) {
    system.add(
        Swarm::<Draggable>::new(),
        Swarm::<Draggable>::subactors(|mut each_draggable| {
            let ui_id = each_draggable.world().id::<UserInterface>();
            let cp_id = each_draggable.world().id::<CurrentPlan>();

            each_draggable.on_create_with(move |_: &InitInteractable, draggable, world| {
                world.send(
                    ui_id,
                    AddInteractable(
                        draggable.id(),
                        AnyShape::Band(Band::new(draggable.path.clone(), 5.0)),
                        4,
                    ),
                );
                Fate::Live
            });

            each_draggable.on(move |_: &ClearInteractable, draggable, world| {
                world.send(ui_id, RemoveInteractable(draggable.id()));
                Fate::Die
            });

            each_draggable.on(move |event, _, world| {
                match *event {
                    Event3d::DragOngoing { from, to, .. } => {
                        world.send(
                            cp_id,
                            ChangeIntent(
                                Intent::MoveSelection(to.into_2d() - from.into_2d()),
                                IntentProgress::Preview,
                            ),
                        );
                    }
                    Event3d::DragFinished { from, to, .. } => {
                        let delta = to.into_2d() - from.into_2d();
                        if delta.norm() < MAXIMIZE_DISTANCE {
                            world.send(
                                cp_id,
                                ChangeIntent(Intent::MaximizeSelection, IntentProgress::Immediate),
                            );
                        } else {
                            world.send(
                                cp_id,
                                ChangeIntent(
                                    Intent::MoveSelection(delta),
                                    IntentProgress::Immediate,
                                ),
                            );
                        }
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

const MAXIMIZE_DISTANCE: N = 0.5;
