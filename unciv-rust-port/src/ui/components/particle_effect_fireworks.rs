use super::particle_effect_animation::{ParticleEffectAnimation, ParticleEffectData, Interpolation, DefaultParticleEffectAnimation};
use ggez::graphics::{self, Vec2, Rect};
use ggez::{Context, GameResult};
use rand::{Rng, seq::SliceRandom};

const EFFECTS_FILE: &str = "effects/fireworks.p";
const AMPLITUDE_X: f32 = 1.0;
const OFFSET_X: f32 = 0.0;
const AMPLITUDE_Y: f32 = 1.0;
const OFFSET_Y: f32 = 0.5;
const TRAVEL_TIME: f32 = 0.6;
const INITIAL_DELAY: f32 = TRAVEL_TIME;
const AMPLITUDE_DELAY: f32 = 0.25;
const MIN_EFFECTS: i32 = 3;
const MAX_EFFECTS: i32 = 5;

lazy_static! {
    static ref INITIAL_END_POINTS: Vec<Vec2> = vec![
        Vec2::new(0.5, 1.5),
        Vec2::new(0.2, 1.0),
        Vec2::new(0.8, 1.0),
    ];
    static ref START_POINT: Vec2 = Vec2::new(0.5, 0.0);
}

/// Fireworks particle effect animation
///
/// This implementation provides fireworks-specific particle effects,
/// with rockets that launch from the bottom and explode in various patterns.
pub struct ParticleEffectFireworks {
    base: DefaultParticleEffectAnimation,
}

impl ParticleEffectFireworks {
    /// Creates a new fireworks particle effect
    pub fn new() -> Self {
        Self {
            base: DefaultParticleEffectAnimation::new(),
        }
    }

    /// Loads the fireworks effect from the default file
    pub fn load(&mut self) -> GameResult {
        self.base.load(EFFECTS_FILE, "ConstructionIcons", INITIAL_END_POINTS.len() as i32)
    }
}

impl ParticleEffectAnimation for ParticleEffectFireworks {
    fn get_target_bounds(&self) -> Rect {
        self.base.get_target_bounds()
    }

    fn configure(&self, effect_data: &mut ParticleEffectData) {
        effect_data.start_point = *START_POINT;

        let mut rng = rand::thread_rng();
        if (0..INITIAL_END_POINTS.len() as i32).contains(&effect_data.index) {
            effect_data.end_point = INITIAL_END_POINTS[effect_data.index as usize];
            effect_data.delay = effect_data.index as f32 * INITIAL_DELAY;
        } else {
            effect_data.end_point = Vec2::new(
                OFFSET_X + AMPLITUDE_X * rng.gen::<f32>(),
                OFFSET_Y + AMPLITUDE_Y * rng.gen::<f32>()
            );
            effect_data.delay = rng.gen::<f32>() * AMPLITUDE_DELAY;
        }

        effect_data.travel_time = TRAVEL_TIME;
        effect_data.interpolation = Interpolation::FastSlow;

        // The file definition has a whole bunch of "explosions" - a "rainbow" and six "shower-color" ones.
        // Show either "rainbow" alone or a random selection of "shower" emitters.
        // It also has some "dazzler" emitters that shouldn't be included in most runs.
        let type_ = rng.gen_range(-1..5);
        if type_ < 0 {
            // Leave only rainbow emitter
            effect_data.effect.remove_emitters(|name| name.starts_with("shower"));
        } else {
            // remove rainbow emitter and [type] "shower-color" emitters
            let mut names: Vec<String> = effect_data.effect.emitters
                .iter()
                .filter(|emitter| emitter.name.starts_with("shower"))
                .map(|emitter| emitter.name.clone())
                .collect();
            names.shuffle(&mut rng);
            let names: Vec<String> = names.into_iter().take(type_ as usize).collect();
            let mut names_set: std::collections::HashSet<String> = names.into_iter().collect();
            names_set.insert("rainbow".to_string());
            effect_data.effect.remove_emitters(|name| names_set.contains(name));
        }
        if rng.gen_range(0..4) > 0 {
            effect_data.effect.remove_emitters(|name| name.starts_with("dazzler"));
        }
    }

    fn on_complete(&self, effect_data: &ParticleEffectData) -> i32 {
        let mut rng = rand::thread_rng();
        if rng.gen_range(0..4) > 0 {
            return 1;
        }
        if self.active_count() <= MIN_EFFECTS as usize {
            return rng.gen_range(1..3);
        }
        if self.active_count() >= MAX_EFFECTS as usize {
            return rng.gen_range(0..2);
        }
        rng.gen_range(0..3)
    }

    fn active_count(&self) -> usize {
        self.base.active_count()
    }

    fn load(&mut self, effects_file: &str, atlas_name: &str, count: i32) -> GameResult {
        self.base.load(effects_file, atlas_name, count)
    }

    fn render(&mut self, ctx: &mut Context, delta: f32) -> GameResult {
        self.base.render(ctx, delta)
    }
}