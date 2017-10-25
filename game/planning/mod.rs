use kay::ActorSystem;

use stagemaster::UserInterfaceID;
use monet::RendererID;

pub mod plan;
pub mod plan_manager;
pub mod materialized_reality;

use self::materialized_reality::MaterializedRealityID;

pub fn setup(
    system: &mut ActorSystem,
    user_interface: UserInterfaceID,
    renderer_id: RendererID,
) -> MaterializedRealityID {
    let materialized_reality = self::materialized_reality::setup(system);
    plan_manager::setup(system, user_interface, renderer_id, materialized_reality);
    materialized_reality
}
