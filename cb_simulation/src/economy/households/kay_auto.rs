//! This is all auto-generated. Do not touch.
#![rustfmt::skip]
#[allow(unused_imports)]
use kay::{ActorSystem, TypedID, RawID, Fate, Actor, TraitIDFrom, ActorOrActorTrait};
#[allow(unused_imports)]
use super::*;

#[derive(Serialize, Deserialize)] #[serde(transparent)]
pub struct HouseholdID {
    _raw_id: RawID
}

impl Copy for HouseholdID {}
impl Clone for HouseholdID { fn clone(&self) -> Self { *self } }
impl ::std::fmt::Debug for HouseholdID {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "HouseholdID({:?})", self._raw_id)
    }
}
impl ::std::hash::Hash for HouseholdID {
    fn hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
        self._raw_id.hash(state);
    }
}
impl PartialEq for HouseholdID {
    fn eq(&self, other: &HouseholdID) -> bool {
        self._raw_id == other._raw_id
    }
}
impl Eq for HouseholdID {}

pub struct HouseholdRepresentative;

impl ActorOrActorTrait for HouseholdRepresentative {
    type ID = HouseholdID;
}

impl TypedID for HouseholdID {
    type Target = HouseholdRepresentative;

    fn from_raw(id: RawID) -> Self {
        HouseholdID { _raw_id: id }
    }

    fn as_raw(&self) -> RawID {
        self._raw_id
    }
}

impl<Act: Actor + Household> TraitIDFrom<Act> for HouseholdID {}

impl HouseholdID {
    pub fn decay(self, dt: Duration, world: &mut World) {
        world.send(self.as_raw(), MSG_Household_decay(dt));
    }
    
    pub fn receive_deal(self, deal: Deal, member: MemberIdx, world: &mut World) {
        world.send(self.as_raw(), MSG_Household_receive_deal(deal, member));
    }
    
    pub fn provide_deal(self, deal: Deal, member: MemberIdx, world: &mut World) {
        world.send(self.as_raw(), MSG_Household_provide_deal(deal, member));
    }
    
    pub fn task_succeeded(self, member: MemberIdx, world: &mut World) {
        world.send(self.as_raw(), MSG_Household_task_succeeded(member));
    }
    
    pub fn task_failed(self, member: MemberIdx, location: RoughLocationID, world: &mut World) {
        world.send(self.as_raw(), MSG_Household_task_failed(member, location));
    }
    
    pub fn reset_member_task(self, member: MemberIdx, world: &mut World) {
        world.send(self.as_raw(), MSG_Household_reset_member_task(member));
    }
    
    pub fn stop_using(self, offer: OfferID, world: &mut World) {
        world.send(self.as_raw(), MSG_Household_stop_using(offer));
    }
    
    pub fn destroy(self, world: &mut World) {
        world.send(self.as_raw(), MSG_Household_destroy());
    }
    
    pub fn on_destroy(self, world: &mut World) {
        world.send(self.as_raw(), MSG_Household_on_destroy());
    }
    
    pub fn update_core(self, current_instant: Instant, world: &mut World) {
        world.send(self.as_raw(), MSG_Household_update_core(current_instant));
    }
    
    pub fn find_new_task_for(self, member: MemberIdx, instant: Instant, location: RoughLocationID, world: &mut World) {
        world.send(self.as_raw(), MSG_Household_find_new_task_for(member, instant, location));
    }
    
    pub fn update_results(self, resource: Resource, update: ResultAspect, world: &mut World) {
        world.send(self.as_raw(), MSG_Household_update_results(resource, update));
    }
    
    pub fn choose_deal(self, world: &mut World) {
        world.send(self.as_raw(), MSG_Household_choose_deal());
    }
    
    pub fn start_trip(self, member: MemberIdx, instant: Instant, world: &mut World) {
        world.send(self.as_raw(), MSG_Household_start_trip(member, instant));
    }
    
    pub fn on_trip_created(self, trip: TripID, world: &mut World) {
        world.send(self.as_raw(), MSG_Household_on_trip_created(trip));
    }
    
    pub fn on_trip_result(self, trip: TripID, result: TripResult, rough_source: RoughLocationID, rough_destination: RoughLocationID, world: &mut World) {
        world.send(self.as_raw(), MSG_Household_on_trip_result(trip, result, rough_source, rough_destination));
    }
    
    pub fn start_task(self, member: MemberIdx, start: Instant, location: RoughLocationID, world: &mut World) {
        world.send(self.as_raw(), MSG_Household_start_task(member, start, location));
    }
    
    pub fn stop_task(self, member: MemberIdx, location: Option < RoughLocationID >, world: &mut World) {
        world.send(self.as_raw(), MSG_Household_stop_task(member, location));
    }
    
    pub fn on_tick(self, current_instant: Instant, world: &mut World) {
        world.send(self.as_raw(), MSG_Household_on_tick(current_instant));
    }
    
    pub fn evaluate(self, offer_idx: OfferIdx, instant: Instant, location: RoughLocationID, requester: EvaluationRequesterID, world: &mut World) {
        world.send(self.as_raw(), MSG_Household_evaluate(offer_idx, instant, location, requester));
    }
    
    pub fn request_receive_deal(self, offer_idx: OfferIdx, requester: HouseholdID, requester_member: MemberIdx, world: &mut World) {
        world.send(self.as_raw(), MSG_Household_request_receive_deal(offer_idx, requester, requester_member));
    }
    
    pub fn request_receive_undo_deal(self, offer_idx: OfferIdx, requester: HouseholdID, requester_member: MemberIdx, world: &mut World) {
        world.send(self.as_raw(), MSG_Household_request_receive_undo_deal(offer_idx, requester, requester_member));
    }
    
    pub fn started_using(self, offer_idx: OfferIdx, user: HouseholdID, using_member: Option < MemberIdx >, world: &mut World) {
        world.send(self.as_raw(), MSG_Household_started_using(offer_idx, user, using_member));
    }
    
    pub fn stopped_using(self, offer_idx: OfferIdx, user: HouseholdID, using_member: Option < MemberIdx >, world: &mut World) {
        world.send(self.as_raw(), MSG_Household_stopped_using(offer_idx, user, using_member));
    }
    
    pub fn started_actively_using(self, offer_idx: OfferIdx, user: HouseholdID, using_member: MemberIdx, world: &mut World) {
        world.send(self.as_raw(), MSG_Household_started_actively_using(offer_idx, user, using_member));
    }
    
    pub fn stopped_actively_using(self, offer_idx: OfferIdx, user: HouseholdID, using_member: MemberIdx, world: &mut World) {
        world.send(self.as_raw(), MSG_Household_stopped_actively_using(offer_idx, user, using_member));
    }
    
    pub fn withdrawal_confirmed(self, offer_idx: OfferIdx, world: &mut World) {
        world.send(self.as_raw(), MSG_Household_withdrawal_confirmed(offer_idx));
    }
    
    pub fn get_ui_info(self, requester: ui :: HouseholdUIID, world: &mut World) {
        world.send(self.as_raw(), MSG_Household_get_ui_info(requester));
    }

    pub fn register_trait(system: &mut ActorSystem) {
        system.register_trait::<HouseholdRepresentative>();
        system.register_trait_message::<MSG_Household_decay>();
        system.register_trait_message::<MSG_Household_receive_deal>();
        system.register_trait_message::<MSG_Household_provide_deal>();
        system.register_trait_message::<MSG_Household_task_succeeded>();
        system.register_trait_message::<MSG_Household_task_failed>();
        system.register_trait_message::<MSG_Household_reset_member_task>();
        system.register_trait_message::<MSG_Household_stop_using>();
        system.register_trait_message::<MSG_Household_destroy>();
        system.register_trait_message::<MSG_Household_on_destroy>();
        system.register_trait_message::<MSG_Household_update_core>();
        system.register_trait_message::<MSG_Household_find_new_task_for>();
        system.register_trait_message::<MSG_Household_update_results>();
        system.register_trait_message::<MSG_Household_choose_deal>();
        system.register_trait_message::<MSG_Household_start_trip>();
        system.register_trait_message::<MSG_Household_on_trip_created>();
        system.register_trait_message::<MSG_Household_on_trip_result>();
        system.register_trait_message::<MSG_Household_start_task>();
        system.register_trait_message::<MSG_Household_stop_task>();
        system.register_trait_message::<MSG_Household_on_tick>();
        system.register_trait_message::<MSG_Household_evaluate>();
        system.register_trait_message::<MSG_Household_request_receive_deal>();
        system.register_trait_message::<MSG_Household_request_receive_undo_deal>();
        system.register_trait_message::<MSG_Household_started_using>();
        system.register_trait_message::<MSG_Household_stopped_using>();
        system.register_trait_message::<MSG_Household_started_actively_using>();
        system.register_trait_message::<MSG_Household_stopped_actively_using>();
        system.register_trait_message::<MSG_Household_withdrawal_confirmed>();
        system.register_trait_message::<MSG_Household_get_ui_info>();
    }

    pub fn register_implementor<Act: Actor + Household>(system: &mut ActorSystem) {
        system.register_implementor::<Act, HouseholdRepresentative>();
        system.add_handler::<Act, _, _>(
            |&MSG_Household_decay(dt), instance, world| {
                instance.decay(dt, world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_Household_receive_deal(ref deal, member), instance, world| {
                instance.receive_deal(deal, member, world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_Household_provide_deal(ref deal, member), instance, world| {
                instance.provide_deal(deal, member, world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_Household_task_succeeded(member), instance, world| {
                instance.task_succeeded(member, world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_Household_task_failed(member, location), instance, world| {
                instance.task_failed(member, location, world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_Household_reset_member_task(member), instance, world| {
                instance.reset_member_task(member, world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_Household_stop_using(offer), instance, world| {
                instance.stop_using(offer, world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_Household_destroy(), instance, world| {
                instance.destroy(world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_Household_on_destroy(), instance, world| {
                instance.on_destroy(world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_Household_update_core(current_instant), instance, world| {
                instance.update_core(current_instant, world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_Household_find_new_task_for(member, instant, location), instance, world| {
                instance.find_new_task_for(member, instant, location, world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_Household_update_results(resource, ref update), instance, world| {
                instance.update_results(resource, update, world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_Household_choose_deal(), instance, world| {
                instance.choose_deal(world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_Household_start_trip(member, instant), instance, world| {
                instance.start_trip(member, instant, world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_Household_on_trip_created(trip), instance, world| {
                instance.on_trip_created(trip, world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_Household_on_trip_result(trip, result, rough_source, rough_destination), instance, world| {
                instance.on_trip_result(trip, result, rough_source, rough_destination, world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_Household_start_task(member, start, location), instance, world| {
                instance.start_task(member, start, location, world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_Household_stop_task(member, location), instance, world| {
                instance.stop_task(member, location, world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_Household_on_tick(current_instant), instance, world| {
                instance.on_tick(current_instant, world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_Household_evaluate(offer_idx, instant, location, requester), instance, world| {
                instance.evaluate(offer_idx, instant, location, requester, world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_Household_request_receive_deal(offer_idx, requester, requester_member), instance, world| {
                instance.request_receive_deal(offer_idx, requester, requester_member, world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_Household_request_receive_undo_deal(offer_idx, requester, requester_member), instance, world| {
                instance.request_receive_undo_deal(offer_idx, requester, requester_member, world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_Household_started_using(offer_idx, user, using_member), instance, world| {
                instance.started_using(offer_idx, user, using_member, world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_Household_stopped_using(offer_idx, user, using_member), instance, world| {
                instance.stopped_using(offer_idx, user, using_member, world)
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_Household_started_actively_using(offer_idx, user, using_member), instance, world| {
                instance.started_actively_using(offer_idx, user, using_member, world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_Household_stopped_actively_using(offer_idx, user, using_member), instance, world| {
                instance.stopped_actively_using(offer_idx, user, using_member, world); Fate::Live
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_Household_withdrawal_confirmed(offer_idx), instance, world| {
                instance.withdrawal_confirmed(offer_idx, world)
            }, false
        );
        
        system.add_handler::<Act, _, _>(
            |&MSG_Household_get_ui_info(requester), instance, world| {
                instance.get_ui_info(requester, world); Fate::Live
            }, false
        );
    }
}

#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Household_decay(pub Duration);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Household_receive_deal(pub Deal, pub MemberIdx);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Household_provide_deal(pub Deal, pub MemberIdx);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Household_task_succeeded(pub MemberIdx);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Household_task_failed(pub MemberIdx, pub RoughLocationID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Household_reset_member_task(pub MemberIdx);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Household_stop_using(pub OfferID);
#[derive(Copy, Clone)] #[allow(non_camel_case_types)]
struct MSG_Household_destroy();
#[derive(Copy, Clone)] #[allow(non_camel_case_types)]
struct MSG_Household_on_destroy();
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Household_update_core(pub Instant);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Household_find_new_task_for(pub MemberIdx, pub Instant, pub RoughLocationID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Household_update_results(pub Resource, pub ResultAspect);
#[derive(Copy, Clone)] #[allow(non_camel_case_types)]
struct MSG_Household_choose_deal();
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Household_start_trip(pub MemberIdx, pub Instant);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Household_on_trip_created(pub TripID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Household_on_trip_result(pub TripID, pub TripResult, pub RoughLocationID, pub RoughLocationID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Household_start_task(pub MemberIdx, pub Instant, pub RoughLocationID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Household_stop_task(pub MemberIdx, pub Option < RoughLocationID >);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Household_on_tick(pub Instant);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Household_evaluate(pub OfferIdx, pub Instant, pub RoughLocationID, pub EvaluationRequesterID);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Household_request_receive_deal(pub OfferIdx, pub HouseholdID, pub MemberIdx);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Household_request_receive_undo_deal(pub OfferIdx, pub HouseholdID, pub MemberIdx);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Household_started_using(pub OfferIdx, pub HouseholdID, pub Option < MemberIdx >);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Household_stopped_using(pub OfferIdx, pub HouseholdID, pub Option < MemberIdx >);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Household_started_actively_using(pub OfferIdx, pub HouseholdID, pub MemberIdx);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Household_stopped_actively_using(pub OfferIdx, pub HouseholdID, pub MemberIdx);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Household_withdrawal_confirmed(pub OfferIdx);
#[derive(Compact, Clone)] #[allow(non_camel_case_types)]
struct MSG_Household_get_ui_info(pub ui :: HouseholdUIID);



#[allow(unused_variables)]
#[allow(unused_mut)]
pub fn auto_setup(system: &mut ActorSystem) {
    HouseholdID::register_trait(system);
    
}