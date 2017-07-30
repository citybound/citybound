use kay::{ActorSystem, Fate};
use stagemaster::geometry::AnyShape;
use super::CurrentPlan;

#[derive(Default)]
pub struct Deselecter;

use super::InitInteractable;
use stagemaster::{UserInterface, AddInteractable};

pub fn setup(system: &mut ActorSystem) {
    system.add(Deselecter::default(), |mut the_deselecter| {
        let deselecter_id = the_deselecter.world().id::<Deselecter>();
        let ui_id = the_deselecter.world().id::<UserInterface>();
        let cp_id = the_deselecter.world().id::<CurrentPlan>();

        the_deselecter.on(move |_: &InitInteractable, _, world| {
            world.send(
                ui_id,
                AddInteractable(deselecter_id, AnyShape::Everywhere, 2),
            );
            Fate::Live
        });

        the_deselecter.on(move |_: &ClearInteractable, _, world| {
            world.send(ui_id, RemoveInteractable(deselecter_id));
            Fate::Die
        });

        the_deselecter.on(move |&event, _, world| {
            if let Event3d::DragFinished { .. } = event {
                world.send(
                    cp_id,
                    ChangeIntent(Intent::Deselect, IntentProgress::Immediate),
                );
            }
            Fate::Live
        })
    });
}

use super::ClearInteractable;
use stagemaster::RemoveInteractable;

use stagemaster::Event3d;
use super::{ChangeIntent, Intent, IntentProgress};
