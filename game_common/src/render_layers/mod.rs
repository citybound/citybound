#[repr(u32)]
pub enum RenderLayers {
    PlanningLotResidentialArea = 100_000_000,
    PlanningLotVacantOutline = 180_000_000,
    PlanningLotOccupiedOutline = 190_000_000,

    LaneAsphalt = 200_000_000,
    LaneMarker = 210_000_000,
    LaneMarkerGaps = 220_000_000,

    PlanningLane = 300_000_000,
    PlanningSwitchLane = 310_000_000,
    PlanningIntersection = 320_000_000,

    Car = 400_000_000,
    TrafficLightBox = 450_000_000,
    TrafficLightLight,
    TrafficLightLightLeft,
    TrafficLightLightRight,

    BuildingField = 500_000_000,
    BuildingWall = 510_000_000,
    BuildingFlatRoof = 520_000_000,
    BuildingBrickRoof = 530_000_000,
    BuildingToBeDestroyed = 590_000_000,

    PlanningGestureLines = 600_000_000,
    PlanningGestureDots = 610_000_000,

    DebugSignalState = 3_004_000_000,
    DebugLandmarkAssociation = 1_005_000_000,
    DebugConnectivity = 1_006_000_000,
    DebugBuildingConnector = 1_007_000_000,
}
