use descartes::{P2, V2, Path, Segment, Band, FiniteCurve, N, RoughlyComparable};
use kay::{ID, CVec, Swarm, CreateWith};
use monet::{Thing};
use core::geometry::{CPath, band_to_thing};
use super::{RoadStrokeRef, RoadStrokeNodeInteractable, InteractableParent};
use super::materialized_reality::BuildableRef;
use super::super::{Lane, TransferLane, AdvertiseForConnectionAndReport};

#[derive(Compact, Clone)]
pub struct RoadStroke{
    nodes: CVec<RoadStrokeNode>,
    _memoized_path: CPath
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct RoadStrokeNodeRef(pub usize, pub usize);

pub const MIN_NODE_DISTANCE : f32 = 1.0;

impl RoadStroke {
    pub fn new(nodes: CVec<RoadStrokeNode>) -> Self {
        if nodes.windows(2).any(|window| window[0].position.is_roughly_within(window[1].position, MIN_NODE_DISTANCE)) {
            panic!("close points in stroke")
        }
        if nodes.len() <= 1 {
            panic!("Invalid stroke")
        }
        RoadStroke{nodes: nodes, _memoized_path: CPath::new(vec![])}
    }

    pub fn nodes(&self) -> &CVec<RoadStrokeNode> {
        &self.nodes
    }

    pub fn nodes_mut(&mut self) -> &mut CVec<RoadStrokeNode> {
        &mut self.nodes
    }

    pub fn path(&self) -> &CPath {
        // TODO: replace by proper Option
        if self._memoized_path.segments().len() == 0 {
            // TODO: maybe there is something less damn dangerous
            #[allow(mutable_transmutes)]
            let unsafe_memoized_path : &mut CPath = unsafe{::std::mem::transmute(&self._memoized_path)};
            *unsafe_memoized_path = Path::new(self.nodes.windows(2).flat_map(|window|
                Segment::biarc(window[0].position, window[0].direction, window[1].position, window[1].direction)
            ).collect::<Vec<_>>())
        }
        &self._memoized_path
    }

    pub fn preview_thing(&self) -> Thing {
        band_to_thing(&Band::new(Band::new(self.path().clone(), 3.0).outline(), 0.3), 0.0)
    }

    #[allow(needless_lifetimes)]
    pub fn create_interactables<'a>(&'a self, self_ref: RoadStrokeRef, class: InteractableParent)
    -> impl Iterator<Item=RoadStrokeNodeInteractable> + 'a {
        self.nodes.iter().enumerate().map(move |(i, node)| {
            let child_ref = match self_ref {
                RoadStrokeRef(stroke_idx) => RoadStrokeNodeRef(stroke_idx, i),
            };
            node.create_interactable(child_ref, class.clone())
        })
    } 

    // TODO: this is really ugly
    pub fn cut_before(&self, offset: N) -> Option<Self> {
        let path = self.path();
        if let Some(cut_path) = path.subsection(0.0, offset) {
            let mut new_nodes = self.nodes.iter().filter(|node|
                cut_path.segments().iter().any(|segment|
                    segment.start().is_roughly_within(node.position, 0.1) || segment.end().is_roughly_within(node.position, 0.1)
                )
            ).cloned().collect::<CVec<_>>();

            if new_nodes.is_empty() {
                None
            } else {
                let new_position = path.along(offset);
                if new_nodes.last().unwrap().position.is_roughly_within(new_position, MIN_NODE_DISTANCE) {
                    if new_nodes.len() == 1 {None}
                    else {Some(RoadStroke::new(new_nodes))}
                } else {
                    new_nodes.push(RoadStrokeNode{
                        position: new_position, direction: path.direction_along(offset)
                    });
                    Some(RoadStroke::new(new_nodes))
                }
            }
        } else {None}
    }

    pub fn cut_after(&self, offset: N) -> Option<Self> {
        let path = self.path();
        if let Some(cut_path) = path.subsection(offset, path.length()) {
            let mut new_nodes = self.nodes.iter().filter(|node|
                cut_path.segments().iter().any(|segment|
                    segment.start().is_roughly_within(node.position, 0.1) || segment.end().is_roughly_within(node.position, 0.1)
                )
            ).cloned().collect::<CVec<_>>();

            if new_nodes.is_empty() {
                None
            } else {
                let new_position = path.along(offset);
                if new_nodes[0].position.is_roughly_within(new_position, MIN_NODE_DISTANCE) {
                    if new_nodes.len() == 1 {None}
                    else {Some(RoadStroke::new(new_nodes))}
                } else {
                    new_nodes.insert(0, RoadStrokeNode{
                        position: new_position, direction: path.direction_along(offset)
                    });
                    Some(RoadStroke::new(new_nodes))
                }
            }
        } else {None}
    }

    pub fn build(&self, report_to: ID, report_as: BuildableRef) {
        Swarm::<Lane>::all() << CreateWith(
            Lane::new(self.path().clone(), match report_as {
                BuildableRef::Intersection(_) => true,
                _ => false,
            }),
            AdvertiseForConnectionAndReport(report_to, report_as)
        );
    }

    pub fn build_transfer(&self, report_to: ID, report_as: BuildableRef) {
        Swarm::<TransferLane>::all() << CreateWith(
            TransferLane::new(self.path().clone()),
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
    pub direction: V2
}

impl RoadStrokeNode {
    pub fn create_interactable(&self, self_ref: RoadStrokeNodeRef, class: InteractableParent) -> RoadStrokeNodeInteractable{
        RoadStrokeNodeInteractable::new(self.position, self.direction, vec![self_ref], class)
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