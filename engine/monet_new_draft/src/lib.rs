pub use descartes::{N, P3, P2, V3, V4, M4, Aff3, Iso3, Persp3, Into2d, Into3d,
                    WithUniqueOrthogonal};
use compact::CVec;
use kay::{World, ActorSystem, External};
use fnv::FnvHashMap;

use glium::backend::glutin::Display;

// IDEAS

// Layer configuration
// - centralised enum for cooperation between mods
// - per-layer decal yes/no
// - per-layer material in the future

// LATER: LOD/freshness per grid cell, varies smoothly
// based on player focus, determines what to stream
// for now just render everything

// Per layer: dynamically combine one-instance geometries based on hotness to save drawcalls


pub struct FrameID(usize);

#[derive(Copy, Clone)]
pub struct TriangleIndices(u16, u16, u16);

#[derive(Compact, Clone, Hash)]
pub struct Mesh {
    pub vertices: CVec<V3>,
    pub indices: CVec<TriangleIndices>,
}

pub struct Instance {
    transform: Aff3,
    color: V3,
}

pub struct RenderTask {
    mesh_hash: MeshHash,
    instances: CVec<Instance>,
    layer_id: u16,
}

pub enum MeshKnowledge {
    Requested(FrameID),
    Known(Mesh),
}

#[derive(Compact, Clone)]
pub struct Renderer {
    id: RendererID,
    mesh_knowledge: FnvHashMap<MeshHash, MeshKnowledge>,
}

pub trait Renderable {
    fn render(&mut self, FrameID: usize) -> (CVec<RenderTask>, Option<CVec<&Mesh>>);
    fn provide_meshes(&mut self) -> CVec<&Mesh>;
}
