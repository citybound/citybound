pub use kay::{External, TypedID};
pub use super::mesh::{Mesh, Instance};

use kay::{ActorSystem, World};
use std::collections::HashMap;
use itertools::Itertools;

pub trait GrouperIndividual {
    fn render_to_grouper(&mut self, grouper: GrouperID, base_individual_id: u32, world: &mut World);
}

#[derive(Clone)]
struct GrouperRendererState {
    n_living_groups: usize,
    n_frozen_groups: usize,
    frozen_up_to_date: bool,
}

pub struct GrouperInner {
    instance_color: [f32; 3],
    base_individual_id: u32,
    is_decal: bool,
    living_individuals: HashMap<GrouperIndividualID, Mesh>,
    frozen_individuals: HashMap<GrouperIndividualID, Mesh>,
    living_groups: Vec<Mesh>,
    frozen_groups: Vec<Mesh>,
    frozen_up_to_date: bool,
    renderer_state: HashMap<RendererID, GrouperRendererState>,
}

#[derive(Compact, Clone)]
pub struct Grouper {
    id: GrouperID,
    inner: External<GrouperInner>,
}

impl ::std::ops::Deref for Grouper {
    type Target = GrouperInner;

    fn deref(&self) -> &GrouperInner {
        &self.inner
    }
}

impl ::std::ops::DerefMut for Grouper {
    fn deref_mut(&mut self) -> &mut GrouperInner {
        &mut self.inner
    }
}

impl Grouper {
    pub fn spawn(
        id: GrouperID,
        instance_color: &[f32; 3],
        base_individual_id: u32,
        is_decal: bool,
        _: &mut World,
    ) -> Grouper {
        Grouper {
            id,
            inner: External::new(GrouperInner {
                instance_color: *instance_color,
                base_individual_id,
                is_decal,
                living_individuals: HashMap::new(),
                frozen_individuals: HashMap::new(),
                living_groups: Vec::new(),
                frozen_groups: Vec::new(),
                frozen_up_to_date: true,
                renderer_state: HashMap::new(),
            }),
        }
    }

    pub fn initial_add(&mut self, id: GrouperIndividualID, world: &mut World) {}

    pub fn update(&mut self, id: GrouperIndividualID, mesh: &Mesh, _: &mut World) {}

    pub fn add_frozen(&mut self, id: GrouperIndividualID, mesh: &Mesh, _: &mut World) {}

    pub fn freeze(&mut self, id: GrouperIndividualID, _: &mut World) {}

    pub fn unfreeze(&mut self, id: GrouperIndividualID, _: &mut World) {}

    pub fn remove(&mut self, id: GrouperIndividualID, _: &mut World) {}

    pub fn clear(&mut self, _: &mut World) {}
}

use {Renderable, RendererID, RenderableID};

impl Renderable for Grouper {
    fn render(&mut self, renderer_id: RendererID, _frame: usize, world: &mut World) {}
}

pub fn setup(system: &mut ActorSystem) {
    system.register::<Grouper>();

    auto_setup(system);
}

mod kay_auto;
pub use self::kay_auto::*;
