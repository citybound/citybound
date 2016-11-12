use descartes::{P2, V2, Path, Segment, Band, FiniteCurve, N, RoughlyComparable};
use kay::{CVec, Swarm, CreateWith};
use monet::{Thing};
use core::geometry::{CPath, band_to_thing};
use super::{PlanRef, RoadStrokeNodeInteractable};

#[derive(Compact, Clone)]
pub struct RoadStroke{
    pub nodes: CVec<RoadStrokeNode>
}

impl RoadStroke {
    pub fn path(&self) -> CPath {
        CPath::new(self.nodes.windows(2).map(|window|
            Segment::line(window[0].position, window[1].position)
        ).collect::<Vec<_>>())
    }

    pub fn preview_thing(&self) -> Thing {
        band_to_thing(&Band::new(Band::new(self.path(), 3.0).outline(), 0.3), 0.0)
    }

    pub fn create_interactables(&self, self_ref: PlanRef) {
        for (i, node) in self.nodes.iter().enumerate() {
            let child_ref = match self_ref {
                PlanRef(stroke_idx, _) => PlanRef(stroke_idx, i)
            };
            node.create_interactables(child_ref);
        }
    } 

    // TODO: this is really ugly
    pub fn cut_before(&self, offset: N) -> Self {
        let path = self.path();
        let cut_path = path.subsection(0.0, offset);
        RoadStroke{nodes: self.nodes.iter().filter(|node|
            cut_path.segments().iter().any(|segment|
                segment.start().is_roughly_within(node.position, 0.1) || segment.end().is_roughly_within(node.position, 0.1)
            )
        ).chain(&[RoadStrokeNode{
            position: path.along(offset), direction: None
        }]).cloned().collect()}
    }

    pub fn cut_after(&self, offset: N) -> Self {
        let path = self.path();
        let cut_path = path.subsection(offset, path.length());
        RoadStroke{nodes: (&[RoadStrokeNode{
            position: path.along(offset), direction: None
        }]).iter().chain(self.nodes.iter().filter(|node|
            cut_path.segments().iter().any(|segment|
                segment.start().is_roughly_within(node.position, 0.1) || segment.end().is_roughly_within(node.position, 0.1)
            )
        )).cloned().collect()}
    }
}

#[derive(Copy, Clone)]
pub struct RoadStrokeNode {
    pub position: P2,
    pub direction: Option<V2>
}

use super::AddToUI;

impl RoadStrokeNode {
    pub fn create_interactables(&self, self_ref: PlanRef) {
        Swarm::<RoadStrokeNodeInteractable>::all() << CreateWith(
            RoadStrokeNodeInteractable::new(self.position, self_ref),
            AddToUI
        );
    }
}