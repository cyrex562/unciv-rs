use bevy::prelude::*;
use bevy_egui::egui;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::ui::screens::basescreen::UncivStage;
use crate::ui::components::tilegroups::TileGroupMap;
use crate::ui::components::widgets::ZoomableScrollPane;

/// A zoomable and pannable map view for cities
pub struct CityMapHolder {
    zoom_pane: ZoomableScrollPane,
    is_panning: AtomicBool,
    is_zooming: AtomicBool,
}

impl CityMapHolder {
    /// Create a new CityMapHolder
    pub fn new() -> Self {
        let mut holder = Self {
            zoom_pane: ZoomableScrollPane::new(20.0, 20.0),
            is_panning: AtomicBool::new(false),
            is_zooming: AtomicBool::new(false),
        };
        holder.setup_zoom_pan_listeners();
        holder
    }

    /// Set up listeners for zoom and pan events
    fn setup_zoom_pan_listeners(&mut self) {
        let is_panning = &self.is_panning;
        let is_zooming = &self.is_zooming;

        // Helper function to update interaction state
        let set_act_hit = move |stage: &mut UncivStage, tile_group_map: &mut TileGroupMap| {
            let is_enabled = !is_zooming.load(Ordering::Relaxed) && !is_panning.load(Ordering::Relaxed);
            stage.set_perform_pointer_enter_exit_events(is_enabled);
            tile_group_map.set_should_act(is_enabled);
            tile_group_map.set_should_hit(is_enabled);
        };

        // Set up pan listeners
        self.zoom_pane.set_pan_start_listener(Box::new(move |stage, tile_group_map| {
            is_panning.store(true, Ordering::Relaxed);
            set_act_hit(stage, tile_group_map);
        }));

        self.zoom_pane.set_pan_stop_listener(Box::new(move |stage, tile_group_map| {
            is_panning.store(false, Ordering::Relaxed);
            set_act_hit(stage, tile_group_map);
        }));

        // Set up zoom listeners
        self.zoom_pane.set_zoom_start_listener(Box::new(move |stage, tile_group_map| {
            is_zooming.store(true, Ordering::Relaxed);
            set_act_hit(stage, tile_group_map);
        }));

        self.zoom_pane.set_zoom_stop_listener(Box::new(move |stage, tile_group_map| {
            is_zooming.store(false, Ordering::Relaxed);
            set_act_hit(stage, tile_group_map);
        }));
    }

    /// Get the underlying zoomable scroll pane
    pub fn zoom_pane(&self) -> &ZoomableScrollPane {
        &self.zoom_pane
    }

    /// Get a mutable reference to the underlying zoomable scroll pane
    pub fn zoom_pane_mut(&mut self) -> &mut ZoomableScrollPane {
        &mut self.zoom_pane
    }

    /// Check if the map is currently being panned
    pub fn is_panning(&self) -> bool {
        self.is_panning.load(Ordering::Relaxed)
    }

    /// Check if the map is currently being zoomed
    pub fn is_zooming(&self) -> bool {
        self.is_zooming.load(Ordering::Relaxed)
    }
}

impl Default for CityMapHolder {
    fn default() -> Self {
        Self::new()
    }
}