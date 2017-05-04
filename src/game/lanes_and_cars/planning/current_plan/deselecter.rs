use kay::{Recipient, Actor, Fate};
use stagemaster::geometry::AnyShape;
use super::CurrentPlan;

#[derive(Default)]
pub struct Deselecter;
impl Actor for Deselecter {}

use super::InitInteractable;
use stagemaster::{UserInterface, AddInteractable};

impl Recipient<InitInteractable> for Deselecter {
    fn receive(&mut self, _msg: &InitInteractable) -> Fate {
        UserInterface::id() << AddInteractable(Self::id(), AnyShape::Everywhere, 2);
        Fate::Live
    }
}

use super::ClearInteractable;
use stagemaster::RemoveInteractable;

impl Recipient<ClearInteractable> for Deselecter {
    fn receive(&mut self, _msg: &ClearInteractable) -> Fate {
        UserInterface::id() << RemoveInteractable(Self::id());
        Fate::Die
    }
}

use stagemaster::Event3d;
use super::{ChangeIntent, Intent, IntentProgress};

impl Recipient<Event3d> for Deselecter {
    fn receive(&mut self, msg: &Event3d) -> Fate {
        match *msg {
            Event3d::DragFinished { .. } => {
                CurrentPlan::id() << ChangeIntent(Intent::Deselect, IntentProgress::Immediate);
                Fate::Live
            }
            _ => Fate::Live,
        }
    }
}

pub fn setup() {
    Deselecter::register_default();
    Deselecter::handle::<InitInteractable>();
    Deselecter::handle::<ClearInteractable>();
    Deselecter::handle::<Event3d>();
}
