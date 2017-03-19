use compact::{CVec, CDict, Compact};
use kay::ID;
use core::read_md_tables::read;
use super::resources::{ResourceId, r};

#[derive(Copy, Clone, Debug)]
enum ConditionKind {
    Equal,
    Less,
    Greater,
}
use self::ConditionKind::{Equal, Less, Greater};

#[derive(Copy, Clone, Debug)]
enum Condition {
    Rs(ActivityParty, ResourceId, ConditionKind, f32),
    Id(ActivityParty, ResourceId, ConditionKind, ActivityParty),
}
use self::Condition::{Rs, Id};

#[derive(Copy, Clone, Debug)]
#[repr(packed)]
struct Rate(ActivityParty, ResourceId, f32);

#[derive(Actor, Compact, Clone, Debug)]
pub struct Place {
    _id: Option<ID>,
    activities: CVec<Activity>,
}

#[derive(Copy, Clone, Debug)]
pub enum ActivityParty {
    Me,
    Us,
    Here,
}
use self::ActivityParty::{Me, Us, Here};

#[derive(Copy, Clone, Debug)]
pub enum Destination {
    Stay,
    Move(ID),
}
use self::Destination::{Stay, Move};

#[derive(Compact, Clone, Debug)]
pub struct Activity {
    //&'static str,
    destination: Destination,
    capacity: u32,
    conditions: CVec<Condition>,
    rates: CVec<Rate>,
}

#[derive(Actor)]
pub struct Family {
    _id: Option<ID>,
    n_members: u8,
    resources: CDict<ResourceId, f32>,
}

pub fn setup() {
    let mut activities = CVec::<Activity>::new();
    for table in &read(&"./src/game/economy/instances/places/appartment.data.md").unwrap() {
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
                if let Ok(rate) = entry.parse::<f32>() {
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
                                    .parse::<f32>()
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

        activities.push(Activity {
            destination: Stay,
            conditions: conditions,
            rates: rates,
            capacity: capacity,
        });
    }

    let loaded_place = Place {
        _id: None,
        activities: activities,
    };

    println!("{:#?}", loaded_place);

    println!("appartment size in bytes: {}, party size is {}, rate size is {}, condition \
                size {}",
             loaded_place.total_size_bytes(),
             ::std::mem::size_of::<ActivityParty>(),
             ::std::mem::size_of::<Rate>(),
             ::std::mem::size_of::<Condition>());
}