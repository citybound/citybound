use kay::ID;
use compact::CVec;
use super::resources::{ResourceMap, ResourceAmount};
use core::simulation::TimeOfDay;

#[derive(Compact, Clone)]
pub struct Deal {
    duration: usize,
    take: ResourceMap<ResourceAmount>,
    give: ResourceMap<ResourceAmount>,
}

#[derive(Compact, Clone, SubActor)]
pub struct Offer {
    _id: Option<ID>,
    by: ID,
    location: ID, // lane
    from: TimeOfDay,
    to: TimeOfDay,
    deal: Deal,
    users: CVec<ID>,
}

#[derive(Copy, Clone)]
pub struct Evaluate {
    pub time: TimeOfDay,
    pub location: ID,
    pub requester: ID
}

use super::resources::ResourceId;

#[derive(Copy, Clone)]
pub struct Find {
    pub time: TimeOfDay,
    pub location: ID,
    pub resource: ResourceId,
    pub requester: ID
}