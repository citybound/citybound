use kay::ID;
use compact::CVec;
use core::simulation::TimeOfDay;

use super::resources::Bag;

#[derive(Compact, Clone, SubActor)]
pub struct Family {
    _id: Option<ID>,
    resources: Bag,
    member_resources: CVec<Bag>,
    member_locations: CVec<ID>,
    favorite_offers: CVec<ID>,
}



impl Family {
    pub fn judge_situation(time: TimeOfDay, member_idx: usize) -> f32 {}
}

#[derive(Compact, Clone, SubActor)]
pub struct Company {
    _id: Option<ID>,
    resources: Bag,
    member_locations: CVec<ID>,
    favorite_offers: CVec<ID>,
    own_offers: CVec<ID>,
}