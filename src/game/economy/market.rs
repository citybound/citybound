use kay::ID;
use compact::CVec;
use super::resources::Bag;
use core::simulation::TimeOfDay;

#[derive(Compact, Clone, SubActor)]
pub struct Offer {
    _id: Option<ID>,
    by: ID,
    location: ID, // lane
    from: TimeOfDay,
    to: TimeOfDay,
    take: Bag,
    give: Bag,
    users: CVec<ID>,
}