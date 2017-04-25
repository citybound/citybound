use kay::ID;
use compact::CVec;
use super::resources::Bag;

#[derive(Compact, Clone, SubActor)]
pub struct Offer {
    _id: Option<ID>,
    by: ID,
    take: Bag,
    give: Bag,
    users: CVec<ID>,
}