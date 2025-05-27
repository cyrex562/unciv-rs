use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttackType {
    Melee {value: u32},
    Direct {value: u32},
    Indirect {value: u32},
    GroundToAir {value: u32},
    AirToGround {value: u32}, // bombing, strafing, missile launches, etc.
    SurfaceToSubsurface {value: u32}, // torpedo, depth charge, etc.; inclues air to subsurface
    SubsurfaceToSurface {value: u32}, // e.g. torpedo
}