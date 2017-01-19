use kay::ID;

#[derive(Copy, Clone, Debug)]
pub struct Interaction {
    pub partner_lane: ID,
    pub start: f32,
    pub partner_start: f32,
    pub kind: InteractionKind,
}

#[derive(Copy, Clone, Debug)]
pub enum InteractionKind {
    Overlap {
        end: f32,
        partner_end: f32,
        kind: OverlapKind,
    },
    Next { green: bool },
    Previous,
}

#[derive(Copy, Clone, Debug)]
pub enum OverlapKind {
    Parallel,
    Transfer,
    Conflicting,
}