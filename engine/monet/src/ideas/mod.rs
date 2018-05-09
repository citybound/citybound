use kay::World;
use fnv::FnvHasher;
use compact::{CHashMap, CVec, COption};
use super::{Geometry, Instance};
use std::hash::{Hash, Hasher};

// Layer configuration
// - centralised enum for cooperation between mods
// - per-layer decal yes/no
// - per-layer material in the future

// LATER: LOD/freshness per grid cell, varies smoothly
// based on player focus, determines what to stream
// for now just render everything

// Per layer: dynamically combine one-instance geometries based on hotness to save drawcalls

#[derive(Compact, Clone)]
pub struct RenderableResult {
    geometry: GeometryHash,
    layer: usize,
    instance: CVec<Instance>,
}

pub trait RenderableNew {
    type InputState: ::std::hash::Hash;
    fn provide_geometries(&mut self, requester: RenderableManagerNew, world: &mut World) ->;
    fn render(&mut self, frame: usize, world: &mut World) -> CVec<RenderableResult>;
    fn input_state_refs(&self) -> &Self::InputState;

    fn input_state_hash(&self) -> RenderableStateHash {
        let mut hasher = FnvHasher::default();
        self.input_state_refs().hash(&mut hasher);
        RenderableStateHash(hasher.finish())
    }
}


#[derive(Compact, Clone)]
pub struct RenderableStateHash(u64);
#[derive(Compact, Clone)]
pub struct GeometryHash(u64);

#[derive(Compact, Clone)]
pub struct GeometryKnowledge {
    first_heard_from: RenderableNewID,
    known_geometry: COption<Geometry>,
}

pub struct RenderableManagerNew {
    id: RenderableManagerNewID,
    rendered_hashes: CHashMap<RenderableNewID, RenderableStateHash>,
    newest_known_hashes: CHashMap<RenderableNewID, RenderableStateHash>,
    geometries: CHashMap<GeometryHash, GeometryKnowledge>,
}

mod kay_auto;
use self::kay_auto::*;