use kay::World;
use compact::CHashMap;
use super::{PlanHistoryUpdate, ProposalID, ProposalUpdate, PlanResultUpdate, ActionGroups};

pub trait PlanningUI {
    fn on_plans_update(
        &mut self,
        master_update: &PlanHistoryUpdate,
        proposal_updates: &CHashMap<ProposalID, ProposalUpdate>,
        _world: &mut World,
    );

    fn on_proposal_preview_update(
        &mut self,
        _proposal_id: ProposalID,
        result_update: &PlanResultUpdate,
        new_actions: &ActionGroups,
        _world: &mut World,
    );
}

pub mod kay_auto;
pub use self::kay_auto::*;
