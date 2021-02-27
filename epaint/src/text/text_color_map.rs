use std::collections::BTreeMap;
use crate::color::Color32;

#[derive(Clone, Debug)]
pub struct TextColorMap {
    color_map:BTreeMap<usize, Color32>,
}

impl TextColorMap {
    pub fn new() -> Self {
	TextColorMap {
	    color_map: BTreeMap::new()
	}
    }
    
    pub fn add_color_change_at_index(&mut self, idx: usize, color:Color32) {
	self.color_map.insert(idx, color);
    }
    
    pub fn adjust(&mut self, adjust_color: &impl Fn(&mut Color32)) {
	for color in self.color_map.iter_mut() {
	    adjust_color(color.1);
	}
    }

    pub fn get_color_change_at_index(&self, idx: usize) -> Option<&Color32> {
	self.color_map.get(&idx)
    }    
}
