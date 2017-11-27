use kay::{ActorSystem, World, External};
use imgui::Ui;
use core::random::{seed, Rng};

use core::simulation::{TimeOfDay, TimeOfDayRange, Instant, Duration, Ticks, SimulationID,
                       Simulatable, SimulatableID, MSG_Simulatable_tick};
use economy::resources::{Resource, Entry};
use economy::market::{Deal, OfferID, EvaluationRequester, EvaluationRequesterID,
                      MSG_EvaluationRequester_expect_n_results, MSG_EvaluationRequester_on_result,
                      EvaluatedSearchResult};
use economy::buildings::BuildingID;
use economy::buildings::rendering::BuildingInspectorID;
use transport::pathfinding::trip::{TripResult, TripListenerID, MSG_TripListener_trip_created,
                                   MSG_TripListener_trip_result};
use transport::pathfinding::RoughLocationID;

pub mod names;
use self::names::{family_name, member_name};

use super::{Household, HouseholdID, HouseholdCore, MemberIdx, MSG_Household_decay,
            MSG_Household_inspect, MSG_Household_provide_deal, MSG_Household_receive_deal,
            MSG_Household_task_succeeded, MSG_Household_task_failed, MSG_Household_destroy,
            MSG_Household_stop_using, MSG_Household_reset_member_task};

#[derive(Compact, Clone)]
pub struct Family {
    id: FamilyID,
    home: BuildingID,
    sleep_offer: OfferID,
    core: HouseholdCore,
}

impl Family {
    pub fn move_into(
        id: FamilyID,
        n_members: usize,
        home: BuildingID,
        simulation: SimulationID,
        world: &mut World,
    ) -> Family {
        simulation.wake_up_in(Ticks(0), id.into(), world);

        let sleep_offer = OfferID::internal(
            id.into(),
            MemberIdx(0),
            home.into(),
            TimeOfDayRange::new(16, 0, 11, 0),
            Deal::new(Some((Resource::Awakeness, 3.0)), Duration::from_hours(1)),
            world,
        );

        // TODO: use sleep offer again

        Family {
            id,
            home,
            sleep_offer,
            core: HouseholdCore::new(n_members, home.into()),
        }
    }
}

use core::simulation::{Sleeper, SleeperID, MSG_Sleeper_wake};

impl Sleeper for Family {
    fn wake(&mut self, current_instant: Instant, world: &mut World) {
        self.update_core(current_instant, world);
    }
}

use super::ResultAspect;

impl EvaluationRequester for Family {
    fn expect_n_results(&mut self, resource: Resource, n: usize, world: &mut World) {
        self.update_results(resource, ResultAspect::SetTarget(n), world);
    }

    fn on_result(&mut self, result: &EvaluatedSearchResult, world: &mut World) {
        let &EvaluatedSearchResult { resource, ref evaluated_deals, .. } = result;
        self.update_results(
            resource,
            ResultAspect::AddDeals(evaluated_deals.clone()),
            world,
        );
    }
}

use transport::pathfinding::trip::{TripListener, TripID};
use super::tasks::TaskState;

impl TripListener for Family {
    fn trip_created(&mut self, trip: TripID, world: &mut World) {
        self.on_trip_created(trip, world);
    }

    fn trip_result(
        &mut self,
        trip: TripID,
        result: TripResult,
        rough_source: RoughLocationID,
        rough_destination: RoughLocationID,
        world: &mut World,
    ) {
        self.on_trip_result(self, trip, result, rough_source, rough_destination, world);
    }
}

impl Household for Family {
    fn core(&self) -> &HouseholdCore {
        &self.core
    }

    fn core_mut(&mut self) -> &mut HouseholdCore {
        &mut self.core
    }

    fn is_shared(resource: Resource) -> bool {
        match resource {
            Resource::Awakeness | Resource::Satiety => false,
            Resource::Money | Resource::Groceries => true,
            _ => unimplemented!(),
        }
    }

    fn supplier_shared(resource: Resource) -> bool {
        match resource {
            Resource::Money => false,
            Resource::Awakeness | Resource::Satiety | Resource::Groceries => true,
            _ => unimplemented!(),
        }
    }

    fn importance(resource: Resource, time: TimeOfDay) -> f32 {
        let hour = time.hours_minutes().0;

        let bihourly_importance = match resource {
            Resource::Awakeness => Some([7, 7, 7, 7, 5, 5, 5, 5, 5, 5, 7, 7]),
            Resource::Satiety => Some([0, 0, 5, 5, 1, 5, 5, 1, 5, 5, 1, 1]),
            Resource::Money => Some([0, 0, 3, 3, 5, 5, 5, 3, 3, 1, 1, 1]),
            Resource::Groceries => Some([0, 0, 4, 4, 1, 4, 4, 4, 4, 4, 0, 0]),
            _ => None,
        };

        bihourly_importance
            .map(|lookup| lookup[hour / 2] as f32)
            .unwrap_or(0.0)
    }

    fn interesting_resources() -> &'static [Resource] {
        &[
            Resource::Awakeness,
            Resource::Satiety,
            Resource::Money,
            Resource::Groceries,
        ]
    }

    fn decay(&mut self, dt: Duration, _: &mut World) {
        for (i, member_resources) in self.member_resources.iter_mut().enumerate() {
            {
                let individuality = seed((self.id, i)).gen_range(0.8, 1.2);
                let awakeness = member_resources.mut_entry_or(Resource::Awakeness, 0.0);
                *awakeness -= 1.0 * individuality * dt.as_hours();
            }
            {
                let individuality = seed((self.id, i, 1u8)).gen_range(0.8, 1.2);
                let satiety = member_resources.mut_entry_or(Resource::Satiety, 0.0);
                if *satiety < 0.0 {
                    let groceries = self.resources.mut_entry_or(Resource::Groceries, 0.0);
                    *groceries -= 3.0;
                    *satiety += 3.0;
                }
                *satiety -= 1.0 * individuality * dt.as_hours();
            }
        }
    }

    fn on_destroy(&mut self, world: &mut World) {
        self.sleep_offer.withdraw_internal(world);
        self.home.remove_household(self.id_as(), world);
    }

    #[allow(useless_format)]
    fn inspect(
        &mut self,
        imgui_ui: &External<Ui<'static>>,
        return_to: BuildingInspectorID,
        world: &mut World,
    ) {
        let ui = imgui_ui.steal();

        ui.window(im_str!("Building")).build(|| {
            ui.tree_node(im_str!("The {} Family:", family_name(self.id)))
                .build(|| {
                    // ui.text(im_str!(
                    //     "({})",
                    //     match self.decision_state {
                    //         DecisionState::None => "",
                    //         DecisionState::Choosing(_, _, _, _) => ": Waiting for choice",
                    //         DecisionState::WaitingForTrip(_) => ": Waiting for trip",
                    //     }
                    // ));
                    for resource in Self::interesting_resources() {
                        if Self::is_shared(*resource) {
                            ui.text(im_str!("{}", resource));
                            ui.same_line(100.0);
                            let amount = self.resources.get(*resource).cloned().unwrap_or(0.0);
                            ui.text(im_str!("{:.2}", amount));
                        }
                    }
                    for (i, (member_resources, member_task)) in
                        self.member_resources
                            .iter()
                            .zip(&self.member_tasks)
                            .enumerate()
                    {
                        ui.spacing();
                        ui.text(im_str!(
                            "{}:",
                            member_name(self.id, MemberIdx(i)),
                        ));
                        ui.text(im_str!(
                            "({} {})",
                            match member_task.state {
                                TaskState::IdleAt(_) => "Idle after getting",
                                TaskState::GettingReadyAt(_) => "Preparing to get",
                                TaskState::InTrip(_) => "In trip to get",
                                TaskState::StartedAt(_, _) => "Getting",
                            },
                            member_task
                                .goal
                                .map(|goal| format!("{}", goal.0))
                                .unwrap_or_else(|| "nothing".to_owned())
                        ));
                        for resource in Self::interesting_resources() {
                            if !Self::is_shared(*resource) {
                                ui.text(im_str!("{}", resource));
                                ui.same_line(100.0);
                                let amount =
                                    member_resources.get(*resource).cloned().unwrap_or(0.0);
                                ui.text(im_str!("{:.2}", amount));
                            }
                        }
                    }
                    ui.tree_node(im_str!("Log")).build(|| for line in self.log
                        .0
                        .lines()
                    {
                        ui.text(im_str!("{}", line));
                    });
                })
        });

        return_to.ui_drawn(ui, world);
    }
}

impl Simulatable for Family {
    fn tick(&mut self, _dt: f32, current_instant: Instant, world: &mut World) {
        self.on_tick(current_instant, world);
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.register::<Family>();
    auto_setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;
