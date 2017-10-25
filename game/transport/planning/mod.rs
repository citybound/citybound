use kay::ActorSystem;

pub mod road_plan;
pub mod materialized_roads;
pub mod lane_stroke;
pub mod road_result_steps;
pub mod current_plan;

pub fn setup(system: &mut ActorSystem) {
    materialized_roads::auto_setup(system);
    current_plan::setup(system);
}