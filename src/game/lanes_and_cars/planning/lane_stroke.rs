use descartes::{P2, V2, Path, Segment, Band, Curve, FiniteCurve, N, RoughlyComparable};
use kay::{ID, CVec, Swarm, CreateWith};
use monet::{Thing};
use core::geometry::{CPath, band_to_thing};
use super::materialized_reality::BuildableRef;
use super::super::{Lane, TransferLane, AdvertiseToTransferAndReport};

#[derive(Compact, Clone)]
pub struct LaneStroke{
    nodes: CVec<LaneStrokeNode>,
    _memoized_path: CPath
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct LaneStrokeNodeRef(pub usize, pub usize);

pub const MIN_NODE_DISTANCE : f32 = 1.0;

impl LaneStroke {
    pub fn new(nodes: CVec<LaneStrokeNode>) -> Self {
        if nodes.windows(2).any(|window| window[0].position.is_roughly_within(window[1].position, MIN_NODE_DISTANCE)) {
            panic!("close points in stroke")
        }
        if nodes.len() <= 1 {
            panic!("Invalid stroke")
        }
        LaneStroke{nodes: nodes, _memoized_path: CPath::new(vec![])}
    }

    pub fn nodes(&self) -> &CVec<LaneStrokeNode> {
        &self.nodes
    }

    pub fn nodes_mut(&mut self) -> &mut CVec<LaneStrokeNode> {
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
        band_to_thing(&Band::new(Band::new(self.path().clone(), 5.0).outline(), 0.3), 0.0)
    }

    // TODO: this is slightly ugly
    pub fn subsection(&self, start: N, end: N) -> Option<Self> {
        let path = self.path();
        if let Some(cut_path) = path.subsection(start, end) {
            let nodes = cut_path.segments().iter().map(|segment|
                LaneStrokeNode{
                    position: segment.start(),
                    direction: segment.start_direction()
                }
            ).chain(cut_path.segments().last().map(|last_segment|
                LaneStrokeNode{
                    position: last_segment.end(),
                    direction: last_segment.end_direction()
                }
            ).into_iter()).collect();
            Some(LaneStroke::new(nodes))
        } else {None}
    }

    pub fn with_subsection_moved(&self, start: N, end: N, delta: V2) -> Option<Self> {
        let nodes_before = self.nodes.iter().take_while(|node|
            self.path().project(node.position).unwrap() < start
        );

        let new_subsection = self.subsection(start, end).into_iter()
            .flat_map(|subsection| subsection.nodes)
            .map(|node| LaneStrokeNode{
                position: node.position + delta,
                direction: node.direction
            });

        let nodes_after = self.nodes.iter().skip_while(|node|
            self.path().project(node.position).unwrap() < end
        );

        let new_segments = nodes_before.cloned()
            .chain(new_subsection)
            .chain(nodes_after.cloned()).collect::<CVec<_>>();

        if new_segments.is_empty() {
            None
        } else {
            Some(LaneStroke::new(new_segments))
        }
     }

    pub fn build(&self, report_to: ID, report_as: BuildableRef) {
        Swarm::<Lane>::all() << CreateWith(
            Lane::new(self.path().clone(), match report_as {
                BuildableRef::Intersection(_) => true,
                _ => false,
            }),
            AdvertiseToTransferAndReport(report_to, report_as)
        );
    }

    pub fn build_transfer(&self, report_to: ID, report_as: BuildableRef) {
        Swarm::<TransferLane>::all() << CreateWith(
            TransferLane::new(self.path().clone()),
            AdvertiseToTransferAndReport(report_to, report_as)
        );
    }
}

impl<'a> RoughlyComparable for &'a LaneStroke {
    fn is_roughly_within(&self, other: &LaneStroke, tolerance: N) -> bool {
        self.nodes.len() == other.nodes.len()
        && self.nodes.iter().zip(other.nodes.iter()).all(|(n1, n2)|
            n1.is_roughly_within(n2, tolerance)
        )
    }
}

#[derive(Copy, Clone)]
pub struct LaneStrokeNode {
    pub position: P2,
    pub direction: V2
}

impl<'a> RoughlyComparable for &'a LaneStrokeNode {
    fn is_roughly_within(&self, other: &LaneStrokeNode, tolerance: N) -> bool {
        self.position.is_roughly_within(other.position, tolerance)
        // && (
        //     (self.direction.is_none() && other.direction.is_none())
        //     || (self.direction.is_some() && other.direction.is_some()
        //         && self.direction.unwrap().is_roughly_within(other.direction.unwrap(), tolerance)))
    }
}