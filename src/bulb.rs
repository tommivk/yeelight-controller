#[derive(Debug, Clone)]
pub struct Bulb {
    pub location: String,
    pub id: String,
    pub power: String,
    pub bright: String,
    pub color_mode: String,
    pub rgb: String,
    pub ct: String,
    pub hue: String,
    pub sat: String,
    pub name: String,
}

impl Bulb {
    pub fn get_id(&self) -> &String {
        &self.id
    }
    pub fn get_location(&self) -> &String {
        &self.location
    }
    pub fn new_bulb() -> Bulb {
        Bulb {
            location: String::new(),
            id: String::new(),
            power: String::new(),
            bright: String::new(),
            color_mode: String::new(),
            rgb: String::new(),
            ct: String::new(),
            hue: String::new(),
            sat: String::new(),
            name: String::new(),
        }
    }
}
