use kay::{Recipient, Actor, Fate};
use ::core::geometry::AnyShape;
use super::CurrentPlan;

#[derive(Default)]
pub struct Deselecter;
impl Actor for Deselecter {}

use super::InitInteractable;
use core::user_interface::Add;

impl Recipient<InitInteractable> for Deselecter {
    fn receive(&mut self, _msg: &InitInteractable) -> Fate {
        ::core::user_interface::UserInterface::id() <<
        Add::Interactable3d(Self::id(), AnyShape::Everywhere, 2);
        Fate::Live
    }
}

use super::ClearInteractable;
use core::user_interface::Remove;

impl Recipient<ClearInteractable> for Deselecter {
    fn receive(&mut self, _msg: &ClearInteractable) -> Fate {
        ::core::user_interface::UserInterface::id() << Remove::Interactable3d(Self::id());
        Fate::Die
    }
}

use core::user_interface::Event3d;
use core::settings::Action;
use super::{ChangeIntent, Intent, IntentProgress};

impl Recipient<Action> for Deselecter {
    fn receive(&mut self, msg: &Action) -> Fate {
        match *msg {
            Action::Event3d(event) => {
                match event {
                    Event3d::DragFinished { .. } => {
                        CurrentPlan::id() <<
                        ChangeIntent(Intent::Deselect, IntentProgress::Finished);
                        Fate::Live
                    }
                    _ => Fate::Live,

                }
            }
            _ => Fate::Live,
        }
    }
}

pub fn setup() {
    Deselecter::register_default();
    Deselecter::handle::<InitInteractable>();
    Deselecter::handle::<ClearInteractable>();
    Deselecter::handle::<Action>();
}