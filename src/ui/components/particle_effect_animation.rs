use ggez::{
    graphics::{self, DrawParam, Rect, SpriteBatch},
    Context, GameResult,
};
use std::collections::HashMap;
use std::path::Path;
use rand::Rng;

/// Represents one currently running particle effect
/// - Points are relative to the bounds provided by get_target_bounds, and would normally stay in the (0..1) range
/// - Time units are seconds
pub struct ParticleEffectData {
    pub index: i32,
    pub effect: ParticleEffect,
    pub start_point: graphics::Vec2,
    pub end_point: graphics::Vec2,
    pub delay: f32,
    pub travel_time: f32,
    pub interpolation: Interpolation,
    accumulated_time: f32,
    percent: f32,
}

impl ParticleEffectData {
    pub fn new(
        index: i32,
        effect: ParticleEffect,
        start_point: graphics::Vec2,
        end_point: graphics::Vec2,
        delay: f32,
        travel_time: f32,
        interpolation: Interpolation,
    ) -> Self {
        Self {
            index,
            effect,
            start_point,
            end_point,
            delay,
            travel_time,
            interpolation,
            accumulated_time: 0.0,
            percent: 0.0,
        }
    }

    pub fn update(&mut self, delta: f32) {
        self.accumulated_time += delta;
        let raw_percent = (self.accumulated_time - self.delay) / self.travel_time;
        self.percent = self.interpolation.apply(raw_percent.clamp(0.0, 1.0));
    }

    pub fn current_x(&self) -> f32 {
        self.start_point.x + self.percent * (self.end_point.x - self.start_point.x)
    }

    pub fn current_y(&self) -> f32 {
        self.start_point.y + self.percent * (self.end_point.y - self.start_point.y)
    }
}

/// Interpolation function for particle movement
pub enum Interpolation {
    Linear,
    FastSlow,
}

impl Interpolation {
    pub fn apply(&self, t: f32) -> f32 {
        match self {
            Interpolation::Linear => t,
            Interpolation::FastSlow => {
                let t2 = t * t;
                t2 / (2.0 * (t2 - t) + 1.0)
            }
        }
    }
}

/// A particle effect that can be loaded from a file and rendered
pub struct ParticleEffect {
    pub emitters: Vec<ParticleEmitter>,
    pub is_complete: bool,
}

impl ParticleEffect {
    pub fn new() -> Self {
        Self {
            emitters: Vec::new(),
            is_complete: false,
        }
    }

    pub fn load(&mut self, effects_file: &Path, atlas: &graphics::Image) -> GameResult {
        // TODO: Implement particle effect loading from file
        // This would parse the .p file format and create emitters
        Ok(())
    }

    pub fn start(&mut self) {
        for emitter in &mut self.emitters {
            emitter.start();
        }
    }

    pub fn update(&mut self, delta: f32) {
        let mut all_complete = true;
        for emitter in &mut self.emitters {
            emitter.update(delta);
            if !emitter.is_complete() {
                all_complete = false;
            }
        }
        self.is_complete = all_complete;
    }

    pub fn draw(&self, ctx: &mut Context, batch: &mut SpriteBatch, x: f32, y: f32) -> GameResult {
        for emitter in &self.emitters {
            emitter.draw(ctx, batch, x, y)?;
        }
        Ok(())
    }

    pub fn scale_effect(&mut self, scale: f32) {
        for emitter in &mut self.emitters {
            emitter.scale(scale);
        }
    }

    pub fn remove_emitters<F>(&mut self, predicate: F)
    where
        F: Fn(&str) -> bool,
    {
        self.emitters.retain(|emitter| !predicate(&emitter.name));
    }
}

/// A particle emitter that generates and manages particles
pub struct ParticleEmitter {
    pub name: String,
    particles: Vec<Particle>,
    // Add other emitter properties as needed
}

impl ParticleEmitter {
    pub fn new(name: String) -> Self {
        Self {
            name,
            particles: Vec::new(),
        }
    }

    pub fn start(&mut self) {
        // Initialize particles
    }

    pub fn update(&mut self, delta: f32) {
        // Update particle positions, lifetimes, etc.
    }

    pub fn draw(&self, ctx: &mut Context, batch: &mut SpriteBatch, x: f32, y: f32) -> GameResult {
        // Draw particles
        Ok(())
    }

    pub fn is_complete(&self) -> bool {
        self.particles.is_empty()
    }

    pub fn scale(&mut self, scale: f32) {
        // Scale particle properties
    }
}

/// Represents a single particle
struct Particle {
    position: graphics::Vec2,
    velocity: graphics::Vec2,
    color: graphics::Color,
    lifetime: f32,
    // Add other particle properties as needed
}

/// Base class for particle effect animations
pub trait ParticleEffectAnimation {
    /// Get the target bounds for drawing
    fn get_target_bounds(&self) -> Rect;

    /// Get the scale factor for the effect
    fn get_scale(&self) -> f32 {
        1.0
    }

    /// Configure a new effect
    fn configure(&self, effect_data: &mut ParticleEffectData) {}

    /// Handle effect completion
    fn on_complete(&self, effect_data: &ParticleEffectData) -> i32 {
        0
    }

    /// Get the number of active effects
    fn active_count(&self) -> usize;

    /// Load the effect from a file
    fn load(&mut self, effects_file: &str, atlas_name: &str, count: i32) -> GameResult;

    /// Render the effect
    fn render(&mut self, ctx: &mut Context, delta: f32) -> GameResult;
}

/// Default implementation of common particle effect animation functionality
pub struct DefaultParticleEffectAnimation {
    template_effect: ParticleEffect,
    max_duration: f32,
    next_index: i32,
    active_effect_data: Vec<ParticleEffectData>,
    target_bounds: Rect,
    last_scale: f32,
}

impl DefaultParticleEffectAnimation {
    pub fn new() -> Self {
        Self {
            template_effect: ParticleEffect::new(),
            max_duration: 0.0,
            next_index: 0,
            active_effect_data: Vec::new(),
            target_bounds: Rect::new(0.0, 0.0, 0.0, 0.0),
            last_scale: 1.0,
        }
    }

    pub fn new_effect(&mut self) {
        let effect = ParticleEffect::new(); // Clone template
        let data = ParticleEffectData::new(
            self.next_index,
            effect,
            graphics::Vec2::new(0.5, 0.5),
            graphics::Vec2::new(0.5, 0.5),
            0.0,
            self.max_duration / 1000.0,
            Interpolation::Linear,
        );
        self.next_index += 1;
        self.active_effect_data.push(data);
    }
}

impl ParticleEffectAnimation for DefaultParticleEffectAnimation {
    fn get_target_bounds(&self) -> Rect {
        self.target_bounds
    }

    fn active_count(&self) -> usize {
        self.active_effect_data.len()
    }

    fn load(&mut self, effects_file: &str, atlas_name: &str, count: i32) -> GameResult {
        self.active_effect_data.clear();
        // TODO: Load atlas and effect file
        for _ in 0..count {
            self.new_effect();
        }
        Ok(())
    }

    fn render(&mut self, ctx: &mut Context, delta: f32) -> GameResult {
        if self.max_duration == 0.0 {
            return Ok(());
        }

        let mut batch = SpriteBatch::new(graphics::Image::new(ctx, "particle.png")?);
        let new_scale = self.get_scale();

        if (new_scale - self.last_scale).abs() > f32::EPSILON {
            let scale_change = new_scale / self.last_scale;
            self.last_scale = new_scale;
            for effect_data in &mut self.active_effect_data {
                effect_data.effect.scale_effect(scale_change);
            }
        }

        let mut repeat_count = 0;
        let mut i = 0;
        while i < self.active_effect_data.len() {
            let effect_data = &mut self.active_effect_data[i];
            effect_data.update(delta);

            let x = self.target_bounds.x + self.target_bounds.w * effect_data.current_x();
            let y = self.target_bounds.y + self.target_bounds.h * effect_data.current_y();

            effect_data.effect.draw(ctx, &mut batch, x, y)?;

            if effect_data.effect.is_complete {
                repeat_count += self.on_complete(effect_data);
                self.active_effect_data.remove(i);
            } else {
                i += 1;
            }
        }

        graphics::draw(ctx, &batch, DrawParam::default())?;

        for _ in 0..repeat_count {
            self.new_effect();
        }

        Ok(())
    }
}