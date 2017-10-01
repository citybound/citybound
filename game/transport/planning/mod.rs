use kay::ActorSystem;

use stagemaster::UserInterfaceID;
use monet::RendererID;
use super::construction::materialized_reality::MaterializedRealityID;

pub mod plan;
pub mod lane_stroke;
pub mod plan_result_steps;
pub mod current_plan;

pub fn setup(
    system: &mut ActorSystem,
    user_interface: UserInterfaceID,
    renderer_id: RendererID,
    materialized_reality: MaterializedRealityID,
) {
    current_plan::setup(system, user_interface, renderer_id, materialized_reality);
}
