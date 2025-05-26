#[derive(Debug,Clone)]
pub enum EffectRange {
    Direct { range: u32},
    Indirect { range: u32 },
    Air { range: u32},
    WaterSurface { range: u32},
    WaterSubsurface { range: u32 },
}

#[derive(Debug,Clone)]
pub enum AreaOfEffect {
    Point,
    Ellipse { semi_major_axis: u32, semi_minor_axis: u32},
    Rectangle { length: u32, width: u32 },
    Line { length: u32},
    Cone { base: u32, height: u32},
    Scatter {min: u32, max: u32}
}

