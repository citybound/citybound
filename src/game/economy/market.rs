use kay::{ActorSystem, ID, Fate};
use kay::swarm::Swarm;
use compact::{CVec, CDict};
use super::resources::{ResourceMap, ResourceId, ResourceAmount};
use super::households::MemberIdx;
use core::simulation::{TimeOfDay, Duration};

#[derive(Compact, Clone)]
pub struct Deal {
    pub duration: Duration,
    pub take: ResourceMap<ResourceAmount>,
    pub give: (ResourceId, ResourceAmount),
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

pub struct Market {
    offers_by_resource: CDict<ResourceId, CVec<ID>>,
}

#[derive(Copy, Clone)]
pub struct Evaluate {
    pub time: TimeOfDay,
    pub location: ID,
    pub requester: ID,
    pub graveness: f32,
}

#[derive(Copy, Clone)]
pub struct Search {
    pub time: TimeOfDay,
    pub location: ID,
    pub resource: ResourceId,
    pub requester: ID,
    pub graveness: f32,
}

#[derive(Compact, Clone)]
pub struct EvaluatedDeal {
    pub offer: ID,
    pub deal: Deal,
    pub possible_until: TimeOfDay,
}

#[derive(Compact, Clone)]
pub struct EvaluatedSearchResult {
    pub n_to_expect: usize,
    pub result: EvaluatedDeal,
}

#[derive(Copy, Clone)]
pub struct GetApplicableDeal(pub ID, pub MemberIdx);

#[derive(Compact, Clone)]
pub struct ApplicableDeal(pub Deal, pub MemberIdx);

pub fn setup(system: &mut ActorSystem) {
    system.add(Swarm::<Offer>::new(),
               Swarm::<Offer>::subactors(|mut each_offer| {
        each_offer.on(|&GetApplicableDeal(id, member), offer, world| {
            world.send(id, ApplicableDeal(offer.deal.clone(), member));
            Fate::Live
        });
    }));
}