use runtime::values::{GribString, GribValue};
use std::collections::HashMap;

pub struct GribHash {
    mutable: bool,
    map: HashMap<GribString, GribValue>,
}

impl GribHash {
    pub fn insert(&mut self, key: GribString, value: GribValue, program: &Program, gc: &mut Gc) {
        //self.map.raw_entry_mut().
        unimplemented!();
    }
}
