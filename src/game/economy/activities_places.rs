use compact::{CVec, CDict, Compact};
use kay::{ID, Recipient, Fate, Actor};
use kay::swarm::{Swarm, SubActor, CreateWith};
use std::collections::HashMap;
use core::read_md_tables::read;

use super::resources::{ResourceId, r};

#[derive(Copy, Clone, Debug)]
pub enum ConditionKind {
    Equal,
    Less,
    Greater,
}
pub use self::ConditionKind::{Equal, Less, Greater};

#[derive(Copy, Clone, Debug)]
pub enum Condition {
    Rs(ActivityParty, ResourceId, ConditionKind, f64),
    Id(ActivityParty, ResourceId, ConditionKind, ActivityParty),
}
pub use self::Condition::{Rs, Id};

#[derive(Copy, Clone, Debug)]
#[repr(packed)]
pub struct Rate(pub ActivityParty, pub ResourceId, pub f64);

#[derive(SubActor, Compact, Clone, Debug)]
pub struct Place {
    _id: Option<ID>,
    pub activities: CVec<Activity>,
}

use super::grid_example::PleaseRegister;
use super::grid_example::RegisterInGrid;

impl Recipient<PleaseRegister> for Place {
    fn receive(&mut self, msg: &PleaseRegister) -> Fate {
        match *msg {
            PleaseRegister(x, y) => {
                super::grid_example::GridExample::id() << RegisterInGrid(self.id(), x, y);
                Fate::Live
            }
        }
    }
}

#[derive(Compact, Clone)]
pub struct AddActivity(pub Activity);

impl Recipient<AddActivity> for Place {
    fn receive(&mut self, msg: &AddActivity) -> Fate {
        match *msg {
            AddActivity(ref activity) => {
                self.activities.push(activity.clone());
                Fate::Live
            }
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum ActivityParty {
    Me,
    Us,
    Here,
}
pub use self::ActivityParty::{Me, Us, Here};

#[derive(Copy, Clone, Debug)]
pub enum Destination {
    Stay,
    Move(ID),
}
pub use self::Destination::{Stay, Move};

#[derive(Compact, Clone, Debug)]
pub struct Activity {
    //&'static str,
    pub destination: Destination,
    pub capacity: u32,
    pub conditions: CVec<Condition>,
    pub rates: CVec<Rate>,
}

#[derive(SubActor, Compact, Clone)]
pub struct Family {
    _id: Option<ID>,
    n_members: u8,
    resources: CDict<ResourceId, f64>,
}

pub fn load_places() -> HashMap<String, Place> {
    let mut places = HashMap::new();

    for table in &read(&"./src/game/economy/instances/places/").unwrap() {
        let c = &table.columns;

        let mut conditions = CVec::new();
        let mut rates = CVec::new();
        let party_strings = [("me", Me), ("us", Us), ("here", Here)];
        let cond_kind_strings = [(">", Greater), ("=", Equal), ("<", Less)];

        for &(party_string, party) in &party_strings {
            for (resource, entry) in c["resource"].iter().zip(&c[party_string]) {
                if entry == "" {
                    continue;
                }
                let r_id = r(resource);
                if let Ok(rate) = entry.parse::<f64>() {
                    rates.push(Rate(party, r_id, rate))
                } else {
                    let id_condition_parsed = party_strings.iter()
                        .any(|&(other_party_string, other_party)| if entry == other_party_string {
                            conditions.push(Id(party, r_id, Equal, other_party));
                            true
                        } else {
                            false
                        });
                    if !id_condition_parsed {
                        let rs_condition_parsed = cond_kind_strings.iter()
                            .any(|&(kind_str, kind)| if &entry[0..1] == kind_str {
                                let val = entry[1..]
                                    .trim()
                                    .parse::<f64>()
                                    .expect(format!("expected a number after {} in {}/{}/{} = {}",
                                                    kind_str,
                                                    table.header,
                                                    table.subheader,
                                                    resource,
                                                    entry)
                                        .as_str());
                                conditions.push(Rs(party, r_id, kind, val));
                                true
                            } else {
                                false
                            });
                        if !rs_condition_parsed {
                            panic!("Weird table entry {}/{}/{} = {}",
                                   table.header,
                                   table.subheader,
                                   resource,
                                   entry);
                        }
                    }
                }
            }
        }

        let capacity = table.subheader
            .split(" x ")
            .nth(1)
            .and_then(|capacity_string| capacity_string.trim().parse::<u32>().ok())
            .expect(format!("expected an activity capacity for {}/{}",
                            table.header,
                            table.subheader)
                .as_str());


        let activity = Activity {
            destination: Stay,
            conditions: conditions,
            rates: rates,
            capacity: capacity,
        };

        places.entry(table.header.clone())
            .or_insert(Place {
                _id: None,
                activities: CVec::new(),
            })
            .activities
            .push(activity);
    }

    places
}

pub fn setup() {
    let loaded_places = load_places();

    println!("{:#?}", loaded_places["Apartment"]);

    println!("appartment size in bytes: {}, party size is {}, rate size is {}, condition \
                size {}",
             loaded_places["Apartment"].total_size_bytes(),
             ::std::mem::size_of::<ActivityParty>(),
             ::std::mem::size_of::<Rate>(),
             ::std::mem::size_of::<Condition>());

    Swarm::<Place>::register_default();
    Swarm::<Place>::handle::<CreateWith<Place, PleaseRegister>>();
    Swarm::<Place>::handle::<AddActivity>();
}