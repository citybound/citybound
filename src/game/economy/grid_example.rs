use kay::{ID, Actor, Recipient, Fate};
use kay::swarm::{Swarm, CreateWith};
use compact::CVec;

use super::activities_places::Place;

pub struct GridExample {
    grid_size: usize,
    grid_ids: Vec<Vec<Option<ID>>>,
    grid_created_count: usize,
}

impl Actor for GridExample {}

extern crate rand;

use self::rand::Rng;

impl Default for GridExample {
    fn default() -> Self {
        let place_templates: Vec<Place> =
            super::activities_places::load_places().into_iter().map(|(_k, v)| v).collect();

        for y in 0..GRID_SIZE {
            for x in 0..GRID_SIZE {
                let place = rand::thread_rng().choose(&place_templates).unwrap();
                Swarm::<Place>::all() << CreateWith(place.clone(), PleaseRegister(x, y));
            }
        }

        GridExample {
            grid_size: GRID_SIZE,
            grid_ids: vec![vec![None; GRID_SIZE]; GRID_SIZE],
            grid_created_count: 0,
        }
    }
}

#[derive(Copy, Clone)]
pub struct PleaseRegister(pub usize, pub usize);
#[derive(Copy, Clone)]
pub struct RegisterInGrid(pub ID, pub usize, pub usize);
use super::activities_places::AddActivity;
use super::pulse::Pulse;

impl Recipient<RegisterInGrid> for GridExample {
    fn receive(&mut self, msg: &RegisterInGrid) -> Fate {
        match *msg {
            RegisterInGrid(id, x, y) => {
                self.grid_ids[y][x] = Some(id);
                self.grid_created_count += 1;
                println!("{}", self.grid_created_count);

                if self.grid_created_count == GRID_SIZE * GRID_SIZE {
                    // start creating roads
                    for y in 0..(GRID_SIZE - 1) {
                        for x in 0..(GRID_SIZE - 1) {
                            let id = self.grid_ids[y][x].unwrap();
                            let id_right = self.grid_ids[y][x + 1].unwrap();
                            let id_below = self.grid_ids[y + 1][x].unwrap();
                            id << AddActivity(road(id_right, GRID_CELL_SIZE));
                            id << AddActivity(road(id_below, GRID_CELL_SIZE));
                            id_right << AddActivity(road(id, GRID_CELL_SIZE));
                            id_below << AddActivity(road(id, GRID_CELL_SIZE));
                            println!("Connected {} {}", x, y);
                        }
                    }

                    self.grid_ids[0][0].unwrap() << Pulse::default();
                }

                Fate::Live
            }
        }
    }
}

use super::resources::r;
use super::activities_places::{Activity, Destination, Rate, Me, Us};

fn road(to: ID, length: f32) -> Activity {
    let time_needed = length as f64 / 500.0/*4000.0*/;
    Activity {
        destination: Destination::Move(to),
        capacity: 100,
        conditions: CVec::new(),
        rates: vec![Rate(Me, r("wakefulness"), time_needed),
                    Rate(Me, r("time"), -time_needed),
                    Rate(Me, r("satiety"), time_needed),
                    Rate(Us, r("petrol"), 4.0 * (length as f64) / 100000.0)]
            .into(),
    }
}

const GRID_SIZE: usize = 3;
const GRID_CELL_SIZE: f32 = 500.0;

pub fn setup() {
    GridExample::register_default();
    GridExample::handle::<RegisterInGrid>();
}