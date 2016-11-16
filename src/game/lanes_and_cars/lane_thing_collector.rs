use monet::{Thing, Instance};
use kay::{ID, Individual, Recipient, ActorSystem, CDict, Fate};

#[derive(Copy, Clone)]
enum ThingLocation{
    Living(usize),
    Frozen(usize)
}

#[derive(Compact, Clone)]
pub struct LaneThingCollector{
    living_things: CDict<ID, Thing>,
    frozen_things: CDict<ID, Thing>,
    cached_frozen_thing: Thing,
    cached_frozen_thing_dirty: bool
}

impl Individual for LaneThingCollector{}

use ::monet::SetupInScene;

impl Recipient<SetupInScene> for LaneThingCollector{
    fn receive(&mut self, _msg: &SetupInScene) -> Fate {
        Fate::Live
    }
}

#[derive(Compact, Clone)]
pub enum Control{
    Update(ID, Thing),
    Freeze(ID),
    Unfreeze(ID),
    Remove(ID)
}

impl Recipient<Control> for LaneThingCollector{
    fn receive(&mut self, msg: &Control) -> Fate {match *msg{
        Control::Update(id, ref thing) => {
            match self.frozen_things.get(id) {
                Some(_) => Fate::Live,
                None => {
                    self.living_things.insert(id, thing.clone());
                    Fate::Live
                }
            }
        },
        Control::Freeze(id) => {
            if let Some(thing) = self.living_things.remove(id) {
                self.frozen_things.insert(id, thing);
                self.cached_frozen_thing_dirty = true;
            }
            Fate::Live
        },
        Control::Unfreeze(id) => {
            if let Some(thing) = self.frozen_things.remove(id) {
                self.living_things.insert(id, thing);
                self.cached_frozen_thing_dirty = true;
            }
            Fate::Live
        },
        Control::Remove(id) => {
            self.living_things.remove(id);
            if let Some(_) = self.frozen_things.remove(id) {
                self.cached_frozen_thing_dirty = true;
            };
            Fate::Live
        }
    }}
}

use ::monet::RenderToScene;
use ::monet::UpdateThing;

#[derive(Copy, Clone)]
pub struct RenderToCollector(pub ID);

impl Recipient<RenderToScene> for LaneThingCollector{
    fn receive(&mut self, msg: &RenderToScene) -> Fate {match *msg{
        RenderToScene{renderer_id, scene_id} => {
            // TODO: this introduces 1 frame delay
            for id in self.living_things.keys() {
                *id << RenderToCollector(LaneThingCollector::id());
            }

            let living_thing = self.living_things.values().sum();

            renderer_id << UpdateThing{
                scene_id: scene_id,
                thing_id: 3498547908345,
                thing: living_thing,
                instance: Instance{
                    instance_position: [0.0, 0.0, -0.1],
                    instance_direction: [1.0, 0.0],
                    instance_color: [0.7, 0.7, 0.7]
                }
            };

            if self.cached_frozen_thing_dirty {
                self.cached_frozen_thing = self.frozen_things.values().sum();
                self.cached_frozen_thing_dirty = false;

                renderer_id << UpdateThing{
                    scene_id: scene_id,
                    thing_id: 9384598345983,
                    thing: self.cached_frozen_thing.clone(),
                    instance: Instance{
                        instance_position: [0.0, 0.0, -0.1],
                        instance_direction: [1.0, 0.0],
                        instance_color: [0.7, 0.7, 0.7]
                    }
                }
            }

            Fate::Live
        }
    }}
}

pub fn setup(system: &mut ActorSystem) {
    system.add_individual(LaneThingCollector{
        living_things: CDict::new(),
        frozen_things: CDict::new(),
        cached_frozen_thing: Thing::new(vec![], vec![]),
        cached_frozen_thing_dirty: false
    });
    system.add_inbox::<Control, LaneThingCollector>();
    system.add_inbox::<SetupInScene, LaneThingCollector>();
    system.add_inbox::<RenderToScene, LaneThingCollector>();
}