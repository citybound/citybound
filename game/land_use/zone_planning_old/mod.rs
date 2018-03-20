use kay::ActorSystem;
use compact::CVec;
use stagemaster::geometry::CShape;

pub mod rendering;
pub mod plan_manager;

#[derive(Copy, Clone)]
enum LandUse {
    Residential,
    Commercial,
    Industrial,
    Agricultural,
    Recreational,
    Official,
}

#[derive(Copy, Clone)]
enum ZoneMeaning {
    LandUse(LandUse),
    MaxHeight(u8),
    SetBack(u8),
}

#[derive(Compact, Clone)]
pub struct Zone {
    meaning: ZoneMeaning,
    shape: CShape,
}

#[derive(Compact, Clone, Default)]
pub struct ZonePlan {
    zones: CVec<Zone>,
}

#[derive(Compact, Clone)]
pub enum ZonePlanAction {
    Add(Zone),
    Remove(Zone),
}

#[derive(Compact, Clone, Default)]
pub struct ZonePlanDelta {
    pub actions: CVec<ZonePlanAction>,
}

impl ZonePlan {
    pub fn with_delta(&self, delta: &ZonePlanDelta) -> Self {
        let mut new_plan = self.clone();

        for action in &delta.actions {
            match *action {
                ZonePlanAction::Add(ref zone) => {
                    if let ZoneMeaning::LandUse(_) = zone.meaning {
                        new_plan.zones.push(zone.clone());
                    } else {
                        unimplemented!()
                    }
                }
                _ => unimplemented!(),
            }
        }

        new_plan
    }

    pub fn get_result(&self) -> ZonePlan {
        self.clone()
    }
}

pub fn setup(system: &mut ActorSystem) {
    plan_manager::setup(system);
    rendering::setup(system);
}