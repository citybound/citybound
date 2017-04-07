mod resources;
mod activities_places;
mod grid_example;
mod pulse;

pub fn setup() {
    resources::setup();
    activities_places::setup();
    grid_example::setup();
    pulse::setup();
}