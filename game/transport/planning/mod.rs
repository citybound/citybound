use kay::ActorSystem;

pub mod road_plan;
pub mod materialized_roads;
pub mod lane_stroke;
pub mod road_result_steps;
pub mod plan_manager;

pub fn setup(system: &mut ActorSystem) {
    materialized_roads::auto_setup(system);
    plan_manager::setup(system);
}
