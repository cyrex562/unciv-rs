pub enum TerrainType {
    Flat,
    Hill,
    Mountain,
    Valley,
    Coast,
    Ocean,
    Lake,
    River,
    Depression,
    Canyon,
    Cliff,
}

pub struct TerrainInfo {
    pub name: String,
    pub terrain_type: TerrainType,
    pub movement_modifier: i32,
    pub defense_modifier: i32,
    pub elevation: i32,
}



lazy_static! {
    pub static ref ALL_TERRAINS: HashMap<TerrainType, TerrainInfo> = {
        use TerrainType::*;
        let mut map = HashMap::new();
        map.insert(
            Flat, TerrainInfo { name: "Flat".to_string(), terrain_type: Flat, movement_modifier: 0, defense_modifier: 0, elevation: 0 }
        );
        map.insert(
            Hill, TerrainInfo { name: "Hill".to_string(), terrain_type: Hill, movement_modifier: -1, defense_modifier: 1, elevation: 1 }
        )
        map.insert(
            Mountain, TerrainInfo { name: "Mountain".to_string(), terrain_type: Mountain, movement_modifier: -3, defense_modifier: 2, elevation: 3 }
        );
        map.insert(
            Valley, TerrainInfo { name: "Valley".to_string(), terrain_type: Valley, movement_modifier: 0, defense_modifier: 0, elevation: 0 }
        );
        
    }
}

