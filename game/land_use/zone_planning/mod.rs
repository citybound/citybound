use compact::CVec;
use stagemaster::geometry::CShape;

#[derive(Copy, Clone)]
enum LandUse {
    Residential,
    Commercial,
    Offices,
    Agricultural,
    Industrial,
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
    Change(Zone),
    Remove(Zone),
}

#[derive(Compact, Clone, Default)]
pub struct ZonePlanDelta {
    actions: CVec<ZonePlanDelta>,
}

impl ZonePlan {
    pub fn with_delta(&self, delta: &ZonePlanDelta) -> Self {
        // TODO
        self.clone()
    }

    pub fn get_result(&self) -> ZonePlan {
        self.clone()
    }
}