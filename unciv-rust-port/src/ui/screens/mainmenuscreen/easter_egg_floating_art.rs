use std::f32::consts::PI;
use std::time::Duration;
use bevy::prelude::*;
use bevy::math::Vec2;
use bevy::sprite::TextureAtlasSprite;
use bevy::time::Timer;
use bevy_egui::{egui, EguiContexts, EguiPlugin};
use rand::Rng;
use rand::distributions::{Distribution, Uniform};

use crate::ui::images::ImageGetter;
use crate::ui::components::widgets::Image;

/// Component that creates floating easter egg images that animate across the screen
pub struct EasterEggFloatingArt {
    images: Vec<Handle<Image>>,
    center_x: f32,
    center_y: f32,
    placement_radius: f32,
    next_image_timer: Timer,
    active_images: Vec<FloatingImage>,
}

/// Represents a single floating image with its animation properties
struct FloatingImage {
    entity: Entity,
    image: Handle<Image>,
    start_pos: Vec2,
    end_pos: Vec2,
    duration: f32,
    elapsed: f32,
    interpolation_type: InterpolationType,
}

/// Types of interpolation for animations
#[derive(Debug, Clone, Copy, PartialEq)]
enum InterpolationType {
    Linear,
    Bounce,
    Swing,
    Smoother,
    FastSlow,
    SlowFast,
}

impl EasterEggFloatingArt {
    /// Creates a new EasterEggFloatingArt
    pub fn new(stage_width: f32, stage_height: f32, name: &str) -> Self {
        // Load all available images
        let mut images = Vec::new();
        for index in 1..=99 {
            let texture_name = format!("EasterEggs/{}{}", name, index);
            if !ImageGetter::image_exists(&texture_name) {
                break;
            }
            images.push(ImageGetter::get_image(&texture_name));
        }

        // Calculate center and placement radius
        let center_x = stage_width / 2.0;
        let center_y = stage_height / 2.0;

        // Find max dimensions of images
        let max_width = images.iter()
            .map(|handle| {
                // In a real implementation, we would get the actual width from the texture
                // For now, we'll use a placeholder value
                100.0
            })
            .fold(0.0, f32::max);

        let max_height = images.iter()
            .map(|handle| {
                // In a real implementation, we would get the actual height from the texture
                // For now, we'll use a placeholder value
                100.0
            })
            .fold(0.0, f32::max);

        // Calculate placement radius
        let placement_radius = (center_x * center_x + center_y * center_y +
                              max_width.powi(2) + max_height.powi(2)).sqrt();

        // Create a timer for the next image
        let mut next_image_timer = Timer::from_seconds(2.0, false);
        next_image_timer.set_elapsed(Duration::from_secs_f32(rand::thread_rng().gen_range(0.0..2.0)));

        Self {
            images,
            center_x,
            center_y,
            placement_radius,
            next_image_timer,
            active_images: Vec::new(),
        }
    }

    /// Spawns the easter egg floating art in the world
    pub fn spawn(&self, commands: &mut Commands) -> Entity {
        commands.spawn((
            Name::new("EasterEggFloatingArt"),
            self.clone(),
        )).id()
    }

    /// Updates the easter egg floating art
    pub fn update(
        &mut self,
        time: Res<Time>,
        mut commands: Commands,
        mut query: Query<(&mut Transform, &mut Sprite, &Handle<Image>)>,
    ) {
        // Update the next image timer
        self.next_image_timer.tick(time.delta());

        // Check if it's time to spawn a new image
        if self.next_image_timer.finished() && !self.images.is_empty() {
            self.next_image(&mut commands);
            self.next_image_timer.reset();
            self.next_image_timer.set_elapsed(Duration::from_secs_f32(
                rand::thread_rng().gen_range(2.0..6.0)
            ));
        }

        // Update active images
        let mut i = 0;
        while i < self.active_images.len() {
            let floating_image = &mut self.active_images[i];
            floating_image.elapsed += time.delta_seconds();

            // Check if the animation is complete
            if floating_image.elapsed >= floating_image.duration {
                // Remove the image
                commands.entity(floating_image.entity).despawn();
                self.active_images.remove(i);
                continue;
            }

            // Calculate the current position based on interpolation
            let t = floating_image.elapsed / floating_image.duration;
            let interpolated_t = match floating_image.interpolation_type {
                InterpolationType::Linear => t,
                InterpolationType::Bounce => self.bounce_interpolation(t),
                InterpolationType::Swing => self.swing_interpolation(t),
                InterpolationType::Smoother => self.smoother_interpolation(t),
                InterpolationType::FastSlow => self.fast_slow_interpolation(t),
                InterpolationType::SlowFast => self.slow_fast_interpolation(t),
            };

            let current_pos = floating_image.start_pos.lerp(floating_image.end_pos, interpolated_t);

            // Update the image position
            if let Ok((mut transform, _, _)) = query.get_mut(floating_image.entity) {
                transform.translation.x = current_pos.x;
                transform.translation.y = current_pos.y;
            }

            i += 1;
        }
    }

    /// Spawns a new floating image
    fn next_image(&mut self, commands: &mut Commands) {
        let mut rng = rand::thread_rng();

        // Select a random image
        let image_index = rng.gen_range(0..self.images.len());
        let image = self.images[image_index].clone();

        // Calculate random angles for start and end positions
        let angle = rng.gen_range(0.0..2.0 * PI);
        let angle2 = angle + rng.gen_range(-0.1667..0.1667) * PI; // +/- 30° so they won't always cross the center exactly

        // Calculate start and end positions
        let offset_x = (self.placement_radius * angle.cos()) as f32;
        let offset_y = (self.placement_radius * angle.sin()) as f32;
        let move_to_x = (self.placement_radius * angle2.cos()) as f32;
        let move_to_y = (self.placement_radius * angle2.sin()) as f32;

        let start_pos = Vec2::new(
            self.center_x - offset_x,
            self.center_y - offset_y
        );

        let end_pos = Vec2::new(
            self.center_x + move_to_x,
            self.center_y + move_to_y
        );

        // Choose a random interpolation type
        let interpolation_type = match rng.gen_range(0..6) {
            1 => InterpolationType::Bounce,
            2 => InterpolationType::Swing,
            3 => InterpolationType::Smoother,
            4 => InterpolationType::FastSlow,
            5 => InterpolationType::SlowFast,
            _ => InterpolationType::Linear,
        };

        // Spawn the image entity
        let entity = commands.spawn((
            SpriteBundle {
                texture: image.clone(),
                transform: Transform::from_xyz(start_pos.x, start_pos.y, 0.0),
                ..default()
            },
        )).id();

        // Add to active images
        self.active_images.push(FloatingImage {
            entity,
            image,
            start_pos,
            end_pos,
            duration: rng.gen_range(3.0..11.0),
            elapsed: 0.0,
            interpolation_type,
        });
    }

    // Interpolation functions
    fn bounce_interpolation(&self, t: f32) -> f32 {
        if t < 0.5 {
            2.0 * t * t
        } else {
            let t = t - 1.0;
            1.0 - 2.0 * t * t
        }
    }

    fn swing_interpolation(&self, t: f32) -> f32 {
        if t < 0.5 {
            2.0 * t * t
        } else {
            let t = t - 1.0;
            1.0 - 2.0 * t * t
        }
    }

    fn smoother_interpolation(&self, t: f32) -> f32 {
        t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
    }

    fn fast_slow_interpolation(&self, t: f32) -> f32 {
        t * t
    }

    fn slow_fast_interpolation(&self, t: f32) -> f32 {
        1.0 - (1.0 - t) * (1.0 - t)
    }
}

// Plugin to add the easter egg floating art to the game
pub struct EasterEggFloatingArtPlugin;

impl Plugin for EasterEggFloatingArtPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_easter_egg_floating_art);
    }
}

// System to update the easter egg floating art
fn update_easter_egg_floating_art(
    mut query: Query<&mut EasterEggFloatingArt>,
    time: Res<Time>,
    mut commands: Commands,
    sprite_query: Query<(&mut Transform, &mut Sprite, &Handle<Image>)>,
) {
    for mut easter_egg in query.iter_mut() {
        easter_egg.update(time, commands, sprite_query);
    }
}