use kay::ActorSystem;

pub mod plan;
pub mod lane_stroke;
pub mod plan_result_steps;
pub mod current_plan;

pub fn setup(system: &mut ActorSystem) {
    current_plan::setup(system);
}
