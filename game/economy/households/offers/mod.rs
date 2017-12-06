use compact::CVec;
use economy::market::Deal;
use super::{HouseholdID, MemberIdx};
use core::simulation::TimeOfDayRange;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct OfferIdx(pub usize);

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct OfferID {
    pub household: HouseholdID,
    pub idx: OfferIdx,
}

#[derive(Compact, Clone)]
pub struct Offer {
    pub offering_member: MemberIdx,
    pub opening_hours: TimeOfDayRange,
    pub deal: Deal,
    pub max_users: usize,
    pub is_internal: bool,
    pub users: CVec<(HouseholdID, Option<MemberIdx>)>,
    pub active_users: CVec<(HouseholdID, MemberIdx)>,
    pub being_withdrawn: bool,
}

impl Offer {
    pub fn new(
        offering_member: MemberIdx,
        opening_hours: TimeOfDayRange,
        deal: Deal,
        max_users: usize,
        is_internal: bool,
    ) -> Offer {
        Offer {
            offering_member,
            opening_hours,
            deal,
            users: CVec::new(),
            active_users: CVec::new(),
            is_internal,
            max_users,
            being_withdrawn: false,
        }
    }
}

//     // The offer stays alive until the withdrawal is confirmed
//     // to prevent offers being used while they're being withdrawn
//     pub fn withdraw(&mut self, world: &mut World) {
//         Market::global_first(world).withdraw(self.deal.main_given(), self.id, world);
//         self.being_withdrawn = true;
//     }

//     // Internal users are manually responsible for forgetting about this offer
//     pub fn withdraw_internal(&mut self, _: &mut World) -> Fate {
//         Fate::Die
//     }
