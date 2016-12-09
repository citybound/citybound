use kay::{ID, Recipient, Actor, Individual, Swarm, ActorSystem, Fate, CreateWith};
use descartes::{Band, Curve, Into2d};
use ::core::geometry::{CPath, AnyShape};

use super::{SelectableStrokeRef, CurrentPlan, PlanControl};

#[derive(Actor, Compact, Clone)]
pub struct LaneStrokeSelectable{
    _id: ID,
    stroke_ref: SelectableStrokeRef,
    path: CPath
}

impl LaneStrokeSelectable{
    pub fn new(stroke_ref: SelectableStrokeRef, path: CPath) -> Self {
        LaneStrokeSelectable{
            _id: ID::invalid(),
            stroke_ref: stroke_ref,
            path: path
        }
    }
}

use super::AddToUI;
use ::core::ui::Add;

impl Recipient<AddToUI> for LaneStrokeSelectable {
    fn receive(&mut self, msg: &AddToUI) -> Fate {match *msg{
        AddToUI => {
            ::core::ui::UserInterface::id() << Add::Interactable3d(
                self.id(),
                AnyShape::Band(Band::new(self.path.clone(), 2.5)),
                1
            );
            Fate::Live
        }
    }}
}

use super::ClearSelectables;
use ::core::ui::Remove;

impl Recipient<ClearSelectables> for LaneStrokeSelectable {
    fn receive(&mut self, msg: &ClearSelectables) -> Fate {match *msg{
        ClearSelectables => {
            ::core::ui::UserInterface::id() << Remove::Interactable3d(self.id());
            Fate::Die
        }
    }}
}

use ::core::ui::Event3d;

impl Recipient<Event3d> for LaneStrokeSelectable {
    fn receive(&mut self, msg: &Event3d) -> Fate {match *msg{
        Event3d::DragOngoing{from, to} => {
            if let (Some(selection_start), Some(selection_end)) =
            (self.path.project(from.into_2d()), self.path.project(to.into_2d())) {
                CurrentPlan::id() << PlanControl::Select(self.stroke_ref, selection_start, selection_end);
            }
            Fate::Live
        },
        _ => Fate::Live
    }}
}


pub fn setup(system: &mut ActorSystem) {
    system.add_individual(Swarm::<LaneStrokeSelectable>::new());
    system.add_inbox::<CreateWith<LaneStrokeSelectable, AddToUI>, Swarm<LaneStrokeSelectable>>();
    system.add_inbox::<ClearSelectables, Swarm<LaneStrokeSelectable>>();
    system.add_inbox::<Event3d, Swarm<LaneStrokeSelectable>>();
}