pub mod lanes_and_cars;

pub fn setup() {
    lanes_and_cars::setup();
}

pub fn setup_ui() {
    lanes_and_cars::lane_rendering::setup();
    lanes_and_cars::planning::current_plan_rendering::setup();
}
