use compact::{CVec, CDict};
use std::collections::HashMap;

struct ResourceRegistry {
    name_to_id: HashMap<String, ResourceId>,
    id_to_info: HashMap<ResourceId, ResourceDescription>,
}

struct ResourceId(u8);
struct ResourceDescription(String, String);
enum Condition {
    Equal(ResourceId, f32),
    Less(ResourceId, f32),
    Greater(ResourceId, f32),
    IDEqual(ResourceId, ID),
}

mod activity_graph {
    #[derive(Actor)]
    pub struct Place {
        _id: ID,
        activities: CVec<Activity>,
    }

    pub struct Activity {
        id: ID,
        destination: Option<ID>,
        conditions: CDict<ResourceId, Condition>,
        rates: CDict<ResourceId, f32>,
        capacity: u32,
    }
}