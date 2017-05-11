use kay::ID;
use compact::CVec;
use core::simulation::TimeOfDay;

use super::resources::{ResourceMap, ResourceId, ResourceAmount, Entry};
use ordered_float::OrderedFloat;

mod judgement_table;
use self::judgement_table::judgement_table;

#[derive(Compact, Clone, SubActor)]
pub struct Family {
    _id: Option<ID>,
    resources: ResourceMap<ResourceAmount>,
    member_resources: CVec<ResourceMap<ResourceAmount>>,
    member_locations: CVec<ID>,
    used_offers: ResourceMap<ID>,
    member_used_offers: CVec<ResourceMap<ID>>,
}

impl Family {
    pub fn top_3_problems(&self, time: TimeOfDay, member_idx: usize) -> Vec<(ResourceId, f32)> {
        let mut resource_graveness = self.resources
            .iter()
            .chain(self.member_resources[member_idx].iter())
            .map(|&Entry(resource, amount)| {
                     (resource, -amount * judgement_table().importance(resource, time))
                 })
            .collect::<Vec<_>>();
        resource_graveness.sort_by_key(|&(_r, i)| OrderedFloat(i));

        resource_graveness.truncate(3);
        resource_graveness
    }
}

#[derive(Compact, Clone, SubActor)]
pub struct Company {
    _id: Option<ID>,
    resources: ResourceMap<ResourceAmount>,
    member_locations: CVec<ID>,
    used_offers: ResourceMap<ID>,
    member_used_offers: CVec<ResourceMap<ID>>,
    own_offers: CVec<ID>,
}
