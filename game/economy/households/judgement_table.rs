use super::super::resources::{r_id, ResourceId, MAX_N_RESOURCE_TYPES};
use core::simulation::TimeOfDay;
use core::read_md_tables;

pub struct JudgementTable([[u8; 12]; MAX_N_RESOURCE_TYPES]);

impl Default for JudgementTable {
    fn default() -> Self {
        JudgementTable([[0; 12]; MAX_N_RESOURCE_TYPES])
    }
}

impl JudgementTable {
    pub fn importance(&self, resource: ResourceId, time: TimeOfDay) -> f32 {
        self.0[resource.as_index()][time.hours_minutes().0 / 2] as f32
    }
}

static mut JUDGEMENT_TABLE: *const JudgementTable = 0 as *const JudgementTable;

pub fn judgement_table() -> &'static JudgementTable {
    unsafe { &*JUDGEMENT_TABLE }
}

pub fn setup() {
    let mut table = Box::<JudgementTable>::default();

    for md_table in read_md_tables::read(&"game/economy/parameters/judgement/adult.data.md")
        .expect("Expected judgement table to exist")
    {
        let c = &md_table.columns;

        for (idx, (resource, resource_id)) in
            c["resource"].iter().map(|s| (s, r_id(s))).enumerate()
        {
            for i in 0..12 {
                table.0[resource_id.as_index()][i] =
                    c.get(&format!("{}h", 2 * i)).expect(&format!(
                        "no entries for {}",
                        resource
                    ))
                        [idx]
                        .parse::<u8>()
                        .expect(&format!("weird entry for {} for {}h", resource, 2 * i));
            }
        }
    }

    unsafe { JUDGEMENT_TABLE = Box::into_raw(table) };
}
