#[repr(usize)]
pub enum Recipients {
    Simulation,
    RenderManager,
    Lane,
}

#[repr(usize)]
pub enum Messages {
    Tick,
    AddSimulatable,
    AddRenderable,
    StartFrame,
    Render,
    InstancePosition,
    AddCar,
}

#[repr(usize)]
#[derive(Copy, Clone)]
pub enum RenderBatches {
    Cars,
}