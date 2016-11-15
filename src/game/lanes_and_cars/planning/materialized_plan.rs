use kay::{ID, CVec, CDict, Actor, Swarm, Recipient, RequestConfirmation, RecipientAsSwarm, CreateWith, ActorSystem, Fate};
use descartes::{P2, Band, Intersect, RoughlyComparable, Norm};
use super::{Plan, PlanRef, RoadStroke, Intersection};

#[derive(Compact, Clone, Actor)]
pub struct MaterializedPlan {
    _id: ID,
    plan: Plan,
    built_lanes: CDict<PlanRef, CVec<ID>>
}

impl MaterializedPlan{
    pub fn new(plan: Plan) -> MaterializedPlan {
        MaterializedPlan {
            _id: ID::invalid(),
            plan: plan,
            built_lanes: CDict::new()
        }
    }
}

#[derive(Copy, Clone)]
pub struct Build;

impl Recipient<Build> for MaterializedPlan {
    fn receive(&mut self, _msg: &Build) -> Fate {
        for (stroke, plan_ref) in self.plan.strokes_after_cutting.iter().enumerate().map(
            |(s, stroke)| (stroke, PlanRef::CutStroke(s))
        ).chain(
            self.plan.intersections.iter().enumerate().flat_map(
                |(i, intersection)| intersection.connecting_strokes.iter().map(
                    move |connecting_stroke| (connecting_stroke, PlanRef::Intersection(i))
                )
            )
        ).filter(|&(stroke, _plan_ref)| stroke.nodes.len() >= 2) {
            stroke.build(self.id(), plan_ref);
        }
        Fate::Live
    }
}

#[derive(Copy, Clone)]
pub struct Unbuild(pub PlanRef);

use super::super::Unbuild as UnbuildLane;

impl Recipient<Unbuild> for MaterializedPlan {
    fn receive(&mut self, msg: &Unbuild) -> Fate {match *msg{
        Unbuild(plan_ref_to_unbuild) => {
            println!("{:?} Unbuilding....", self.id());
            println!("trying: {:?} {:?}", plan_ref_to_unbuild, &**self.built_lanes.get(plan_ref_to_unbuild).unwrap());
            for lane_to_unbuild in self.built_lanes.remove(plan_ref_to_unbuild).unwrap().iter() {
                println!("removed! {:?} {:?}", *lane_to_unbuild, plan_ref_to_unbuild);
                *lane_to_unbuild << UnbuildLane;
            }
            Fate::Live
        }
    }}
}

#[derive(Compact, Clone)]
pub struct CollectIntersectionPoints{
    pub requester: ID,
    pub other_strokes: CVec<RoadStroke>,
    pub other_points: CVec<P2>
}

#[derive(Compact, Clone)]
pub struct ChangesAndIntersectionPoints{
    pub affected_plan_id: ID,
    pub replaced_intersections: CVec<PlanRef>,
    pub points: CVec<P2>
}

impl Recipient<CollectIntersectionPoints> for MaterializedPlan {
    fn receive(&mut self, msg: &CollectIntersectionPoints) -> Fate {match *msg{
        CollectIntersectionPoints{requester, ref other_strokes, ref other_points} => {
            let mut all_intersection_points = Vec::new();
        
            for stroke1 in self.built_cut_strokes() {
                if stroke1.nodes.len() > 1 {
                    let band1 = Band::new(stroke1.path(), 8.0).outline();
                    for stroke2 in other_strokes.iter() {
                        if stroke2.nodes.len() > 1 {
                            let band2 = Band::new(stroke2.path(), 8.0).outline();

                            let intersections = (&band1, &band2).intersect();
                            all_intersection_points.extend(intersections.iter().map(|i| i.position));
                        }
                    }
                }
            }

            let mut replaced_intersections = CVec::new();

            for (index, intersection) in self.plan.intersections.iter().enumerate() {
                let close_to_shape = |point: &P2| {
                    intersection.points.iter().any(|other_point: &P2| (*point - *other_point).norm() < super::INTERSECTION_GROUPING_RADIUS)
                };

                let stroke_intersects = |stroke: &RoadStroke| {
                    if stroke.nodes.len() > 1 {
                        let band = Band::new(stroke.path(), 3.0).outline();
                        let no_intersections = (&intersection.shape, &band).intersect().is_empty();
                        !no_intersections
                    } else {false}
                };

                if other_points.iter().chain(all_intersection_points.iter()).any(close_to_shape)
                || other_strokes.iter().any(stroke_intersects) { 
                    all_intersection_points.extend(intersection.points.iter());
                    replaced_intersections.push(PlanRef::Intersection(index));
                }
            }

            requester << ChangesAndIntersectionPoints{
                affected_plan_id: self.id(),
                replaced_intersections: replaced_intersections,
                points: all_intersection_points.into()
            };

            Fate::Live
        }
    }}
}

#[derive(Compact, Clone)]
pub struct IntersectWith{
    pub requester: ID,
    pub new_intersections: CVec<Intersection>,
    pub replaced_intersections: CVec<(ID, PlanRef)>
}

#[derive(Compact, Clone)]
pub struct ChangesAfterIntersecting{
    pub affected_plan_id: ID,
    pub updated_intersections: CVec<Intersection>,
    pub new_cut_strokes: CVec<RoadStroke>,
    pub cut_strokes_to_debuild: CVec<PlanRef>
}

use super::cut_strokes_at_intersections;

impl Recipient<IntersectWith> for MaterializedPlan {
    fn receive(&mut self, msg: &IntersectWith) -> Fate {match *msg{
        IntersectWith{requester, ref new_intersections, ref replaced_intersections} => {
            let relevant_own_intersections : CVec<_> = self.plan.intersections.iter().enumerate()
                .filter_map(|(index, intersection)|
                    if replaced_intersections.contains(&(self.id(), PlanRef::Intersection(index))) {
                        None
                    } else {
                        Some(intersection.clone())
                    }).collect();
            let mut combined_intersections : CVec<_> = new_intersections.iter().cloned().chain(relevant_own_intersections.iter().cloned()).collect();
            let all_new_cut_strokes = cut_strokes_at_intersections(&self.plan.strokes, &mut combined_intersections);

            let updated_intersections : CVec<_> = combined_intersections[..new_intersections.len()].iter().cloned().collect();

            let only_changed_strokes : CVec<_> = all_new_cut_strokes.iter().filter(|new_cut_stroke|
                !self.plan.strokes_after_cutting.iter().any(|old_stroke|
                    old_stroke.is_roughly_within(new_cut_stroke, 0.1)
                )
            ).cloned().collect();

            let cut_strokes_to_debuild = self.built_cut_strokes().enumerate().filter_map(|(i, stroke1)|
                if stroke1.nodes.len() > 1 {
                    let band1 = Band::new(stroke1.path(), 3.0).outline();
                    let conflicts_with_changed = only_changed_strokes.iter().any(|stroke2|
                        if stroke2.nodes.len() > 1 {
                            let band2 = Band::new(stroke2.path(), 3.0).outline();

                            ! (&band1, &band2).intersect().is_empty()
                        } else {false}
                    );
                    if conflicts_with_changed {
                        Some(PlanRef::CutStroke(i))
                    } else {None}
                } else {None}
            ).collect();

            requester << ChangesAfterIntersecting{
                affected_plan_id: self.id(),
                updated_intersections: updated_intersections,
                new_cut_strokes: only_changed_strokes,
                cut_strokes_to_debuild: cut_strokes_to_debuild
            };

            Fate::Live
        }
    }}
}

#[derive(Copy, Clone)]
pub struct ReportLaneBuilt(pub ID, pub PlanRef);

impl Recipient<ReportLaneBuilt> for MaterializedPlan {
    fn receive(&mut self, msg: &ReportLaneBuilt) -> Fate {match *msg{
        ReportLaneBuilt(lane_id, plan_ref) => {
            println!("{:?}: {:?} built as {:?}", self.id(), lane_id, plan_ref);
            self.built_lanes.push_or_create_at(plan_ref, lane_id);
            Fate::Live
        }
    }}
} 

use monet::SetupInScene;

impl RecipientAsSwarm<SetupInScene> for MaterializedPlan {
    fn receive(_swarm: &mut Swarm<MaterializedPlan>, _msg: &SetupInScene) -> Fate {
        Fate::Live
    }
}

impl MaterializedPlan{
    #[allow(needless_lifetimes)]
    fn built_cut_strokes<'a>(&'a self) -> impl Iterator<Item=&'a RoadStroke> + 'a {
        self.built_lanes.keys().filter_map(move |plan_ref|
            if let PlanRef::CutStroke(index) = *plan_ref {
                Some(&self.plan.strokes_after_cutting[index])
            } else {None}
        )
    }
}

pub fn setup(system: &mut ActorSystem) {
    system.add_individual(Swarm::<MaterializedPlan>::new());
    system.add_inbox::<CreateWith<MaterializedPlan, Build>, Swarm<MaterializedPlan>>();
    system.add_inbox::<ReportLaneBuilt, Swarm<MaterializedPlan>>();
    system.add_inbox::<Unbuild, Swarm<MaterializedPlan>>();
    system.add_inbox::<RequestConfirmation<CollectIntersectionPoints>, Swarm<MaterializedPlan>>();
    system.add_inbox::<RequestConfirmation<IntersectWith>, Swarm<MaterializedPlan>>();
}