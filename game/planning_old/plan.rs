use descartes::{N, RoughlyComparable};
use compact::CDict;
use itertools::Itertools;

use transport::transport_planning_old::road_plan::{RoadPlan, RoadPlanDelta, RoadPlanResult,
                                                   RoadPlanResultDelta};
use land_use::buildings::{BuildingPlanResultDelta, MaterializedBuildings};
use land_use::zone_planning_old::{ZonePlan, ZonePlanDelta};

#[derive(Clone, Compact, Default)]
pub struct Plan {
    pub roads: RoadPlan,
    pub zones: ZonePlan,
}

impl Plan {
    pub fn with_delta(&self, delta: &PlanDelta) -> Self {
        Plan {
            roads: self.roads.with_delta(&delta.roads),
            zones: self.zones.with_delta(&delta.zones),
        }
    }

    pub fn get_result(&self) -> PlanResult {
        PlanResult {
            roads: self.roads.get_result(),
            zones: self.zones.get_result(),
        }
    }
}

#[derive(Compact, Clone, Default)]
pub struct PlanDelta {
    pub roads: RoadPlanDelta,
    pub zones: ZonePlanDelta,
}

#[derive(Compact, Clone, Default)]
pub struct PlanResult {
    pub roads: RoadPlanResult,
    pub zones: ZonePlan,
}

#[derive(Compact, Clone, Default)]
pub struct PlanResultDelta {
    pub roads: RoadPlanResultDelta,
    pub buildings: BuildingPlanResultDelta,
}

impl PlanResult {
    pub fn delta(
        &self,
        old: &Self,
        materialized_buildings: &MaterializedBuildings,
    ) -> PlanResultDelta {
        let road_result_delta = self.roads.delta(&old.roads);
        PlanResultDelta {
            buildings: materialized_buildings.delta_with_road_result_delta(&road_result_delta),
            roads: road_result_delta,
        }
    }
}

#[derive(Compact, Clone)]
pub struct ReferencedDelta<Ref: Copy + Eq, T: ::compact::Compact + Clone> {
    pub to_create: CDict<Ref, T>,
    pub to_destroy: CDict<Ref, T>,
    pub old_to_new: CDict<Ref, Ref>,
}

impl<Ref: Copy + Eq, T: ::compact::Compact + Clone> Default for ReferencedDelta<Ref, T> {
    fn default() -> Self {
        ReferencedDelta {
            to_create: CDict::new(),
            to_destroy: CDict::new(),
            old_to_new: CDict::new(),
        }
    }
}

impl<Ref: Copy + Eq, T: ::compact::Compact + Clone> ReferencedDelta<Ref, T> {
    pub fn compare<'a, F: Fn(&'a T, &'a T) -> bool>(
        new: &'a CDict<Ref, T>,
        old: &'a CDict<Ref, T>,
        equivalent: F,
    ) -> Self {
        let pairs = new.pairs().cartesian_product(old.pairs());
        let old_to_new = pairs
            .filter_map(|pair| match pair {
                ((new_ref, new), (old_ref, old)) => {
                    if equivalent(new, old) {
                        Some((*old_ref, *new_ref))
                    } else {
                        None
                    }
                }
            })
            .collect::<CDict<_, _>>();

        let to_create = new.pairs()
            .filter_map(|(new_ref, new)| if old_to_new.values().any(
                |not_really_new_ref| not_really_new_ref == new_ref,
            )
            {
                None
            } else {
                Some((*new_ref, new.clone()))
            })
            .collect();

        let to_destroy = old.pairs()
            .filter_map(|(old_ref, old)| {
                let has_revived = old_to_new.keys().any(|revived_old_ref| {
                    revived_old_ref == old_ref
                });
                if has_revived {
                    None
                } else {
                    Some((*old_ref, old.clone()))
                }
            })
            .collect();

        ReferencedDelta {
            to_create: to_create,
            to_destroy: to_destroy,
            old_to_new: old_to_new,
        }
    }

    pub fn compare_roughly<'a>(new: &'a CDict<Ref, T>, old: &'a CDict<Ref, T>, tolerance: N) -> Self
    where
        &'a T: RoughlyComparable + 'a,
    {
        Self::compare(new, old, |new_item, old_item| {
            new_item.is_roughly_within(old_item, tolerance)
        })
    }
}
