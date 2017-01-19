pub mod lane;
pub mod connectivity;
pub mod construction;
pub mod microtraffic;
pub mod rendering;

pub mod planning;
pub mod pathfinding;

pub fn setup() {
    self::lane::setup();
    self::construction::setup();
    self::microtraffic::setup();
    self::pathfinding::setup();
}

pub fn setup_ui() {
    rendering::setup();
    planning::setup();
    planning::current_plan_rendering::setup();
}