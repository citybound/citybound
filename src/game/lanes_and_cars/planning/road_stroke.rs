use descartes::{P2, V2, Path, Segment, Band, FiniteCurve, N, RoughlyComparable};
use kay::{ID, CVec, Swarm, CreateWith};
use monet::{Thing};
use core::geometry::{CPath, band_to_thing};
use super::{RoadStrokeRef, RoadStrokeNodeInteractable};
use super::materialized_reality::BuildableRef;
use super::super::{Lane, AdvertiseForConnectionAndReport};

#[derive(Compact, Clone)]
pub struct RoadStroke{
    pub nodes: CVec<RoadStrokeNode>
}

#[derive(Copy, Clone)]
pub struct RoadStrokeNodeRef(pub usize, pub usize);

impl RoadStroke {
    pub fn path(&self) -> CPath {
        if self.nodes.len() == 1 {
            CPath::new(vec![Segment::line(self.nodes[0].position, self.nodes[0].position)])
        } else {
            CPath::new(self.nodes.windows(2).map(|window|
                Segment::line(window[0].position, window[1].position)
            ).collect::<Vec<_>>())
        }
    }

    pub fn preview_thing(&self) -> Thing {
        band_to_thing(&Band::new(Band::new(self.path(), 3.0).outline(), 0.3), 0.0)
    }

    pub fn create_interactables(&self, self_ref: RoadStrokeRef) {
        for (i, node) in self.nodes.iter().enumerate() {
            let child_ref = match self_ref {
                RoadStrokeRef(stroke_idx) => RoadStrokeNodeRef(stroke_idx, i),
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

    pub fn build(&self, report_to: ID, report_as: BuildableRef) {
        Swarm::<Lane>::all() << CreateWith(
            Lane::new(self.path()),
            AdvertiseForConnectionAndReport(report_to, report_as)
        );
    }
}

impl<'a> RoughlyComparable for &'a RoadStroke {
    fn is_roughly_within(&self, other: &RoadStroke, tolerance: N) -> bool {
        self.nodes.len() == other.nodes.len()
        && self.nodes.iter().zip(other.nodes.iter()).all(|(n1, n2)|
            n1.is_roughly_within(n2, tolerance)
        )
    }
}

#[derive(Copy, Clone)]
pub struct RoadStrokeNode {
    pub position: P2,
    pub direction: Option<V2>
}

use super::AddToUI;

impl RoadStrokeNode {
    pub fn create_interactables(&self, self_ref: RoadStrokeNodeRef) {
        Swarm::<RoadStrokeNodeInteractable>::all() << CreateWith(
            RoadStrokeNodeInteractable::new(self.position, self_ref),
            AddToUI
        );
    }
}

impl<'a> RoughlyComparable for &'a RoadStrokeNode {
    fn is_roughly_within(&self, other: &RoadStrokeNode, tolerance: N) -> bool {
        self.position.is_roughly_within(other.position, tolerance)
        // && (
        //     (self.direction.is_none() && other.direction.is_none())
        //     || (self.direction.is_some() && other.direction.is_some()
        //         && self.direction.unwrap().is_roughly_within(other.direction.unwrap(), tolerance)))
    }
}