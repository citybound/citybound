use kay::{Recipient, Fate, Actor, ID};
use kay::swarm::{Swarm, SubActor};
use compact::CDict;
use super::resources::{r, ResourceId};

#[derive(Compact, Clone, Debug)]
pub struct Pulse {
    me: CDict<ResourceId, f64>,
    us: CDict<ResourceId, f64>,
    age: u32, 
    //history: CVec<Activity>,
}

impl Default for Pulse {
    fn default() -> Self {
        let mut just_time = CDict::new();
        just_time.insert(r("time"), 24.0);

        Pulse {
            me: just_time,
            us: CDict::new(),
            age: 0, 
            //history: CVec::new(),
        }
    }
}

use super::activities_places::Place;
use super::activities_places::{Rs, Id, ConditionKind, ActivityParty, Me, Us, Here, Equal, Less,
                               Greater, Stay, Move, Rate};

impl Recipient<Pulse> for Place {
    fn receive(&mut self, msg: &Pulse) -> Fate {
        match *msg {
            Pulse { ref me, ref us, age /*, ref history*/ } => {
                for activity in &self.activities {
                    let condition_error =
                        activity.conditions.iter().flat_map(|condition| match *condition {
                            Rs(party, resource, cond, val) => {
                                let current_val_source = match party {
                                    Me => Some(&me),
                                    Us => Some(&us),
                                    Here => None,
                                };
                                let current_val =
                                    current_val_source.and_then(|source| source.get(resource).cloned()).unwrap_or(0.0);
                                match cond {
                                    Equal => if current_val == val {None} else {Some(1/*format!("expected {:?} with value {} to equal {}", resource, current_val, val)*/)},
                                    Less => if current_val < val {None} else {Some(2/*format!("expected {:?} with value {} to be < {}", resource, current_val, val)*/)},
                                    Greater => if current_val > val {None} else {Some(3/*format!("expected {:?} with value {} to be > {}", resource, current_val, val)*/)},
                                }
                            }
                            Id(party, resource, cond, target_party) => {
                                let current_val_source = match party {
                                    Me => Some(&me),
                                    Us => Some(&us),
                                    Here => None,
                                };
                                let target_id = match target_party {
                                    Here => self.id(),
                                    _ => unimplemented!()
                                };
                                let current_val =
                                    current_val_source.and_then(|source| source.get(resource).cloned()).map(|val_n|
                                        unsafe {::std::mem::transmute::<f64, ID>(val_n)}
                                    );

                                match cond {
                                    Equal => if current_val.is_some() && current_val.unwrap() == target_id {None} else {Some(4/*format!("expected {:?} with value {:?} to equal {:?}", resource, current_val, target_id)*/)},
                                    Less => unreachable!(),
                                    Greater => unreachable!(),
                                }
                            },
                        }).next();

                    let mut new_pulse = Pulse {
                        me: me.clone(),
                        us: us.clone(),
                        age: age, 
                        // history: history.clone(),
                    };

                    if let Some(error) = condition_error {
                        //if me.values().next().unwrap() < &20.0 {
                        println!("Pulse ended at {:?} (conditions): {}, t = {}, age = {}",
                                 self.id(),
                                 error,
                                 me.values().next().unwrap(),
                                 age);
                        //}
                        continue;
                    }

                    let mut rate_error = None;

                    for &Rate(party, resource, rate) in &activity.rates {
                        let maybe_target = match party {
                            Me => Some(&mut new_pulse.me),
                            Us => Some(&mut new_pulse.us),
                            Here => None,
                        };

                        if let Some(target) = maybe_target {
                            if let Some(current) = target.get(resource).cloned() {
                                if current + rate < 0.0 {
                                    rate_error = Some(format!("{:?} became less than 0", resource));
                                }
                                target.insert(resource, current + rate);
                            } else {
                                target.insert(resource, rate);
                                if rate < 0.0 {
                                    rate_error = Some(format!("{:?} was 0", resource));
                                }
                            }
                        }
                    }

                    if let Some(error) = rate_error {
                        if me.values().next().unwrap() < &20.0 {
                            println!("Pulse ended at {:?} (rates): {}, t = {}, age = {}",
                                     self.id(),
                                     error,
                                     me.values().next().unwrap(),
                                     age);
                        }
                    } else {
                        // new_pulse.history.push(activity.clone());
                        new_pulse.age += 1;

                        let next_recipient = match activity.destination {
                            Stay => self.id(),
                            Move(id) => id,
                        };

                        next_recipient << new_pulse;
                    }
                }
                Fate::Live
            }
        }
    }
}

pub fn setup() {
    Swarm::<Place>::handle::<Pulse>();
}