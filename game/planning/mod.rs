use kay::ActorSystem;

use stagemaster::UserInterfaceID;
use monet::RendererID;

pub mod plan;
pub mod current_plan;
pub mod materialized_reality;

pub fn setup(system: &mut ActorSystem, user_interface: UserInterfaceID, renderer_id: RendererID) {
    let materialized_reality = self::materialized_reality::setup(system);
    current_plan::setup(system, user_interface, renderer_id, materialized_reality);
}
