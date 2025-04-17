use std::iter::Iterator;
use std::iter::Map;
use std::iter::Rev;
use std::ops::RangeInclusive;

use crate::logic::civilization::Civilization;
use crate::ui::components::tilegroups::TileSetStrings;
use crate::ui::images::image_getter::ImageGetter;

/// A metaprogramming class for short-circuitingly finding the first existing of multiple image options
/// according to `ImageGetter::image_exists`.
///
/// Has a `try_image` method that can be chain-called with functions which return candidate image paths.
/// The first function to return a valid image path stops subsequently chained calls from having any effect,
/// and its result is saved to be retrieved by the `get_path` and `get_image` methods at the end of the candidate chain.
///
/// (So it is similar to Iterator in that intermediate "transforms" are only evaluated when necessary,
/// but it is also different as resolution happens early, while chaining, not triggered by a terminating transform.)
///
/// Binds candidate functions to a `scope` instance of type `T` provided to primary constructor, for syntactic convenience.
/// Bind to `()` when not needed.
///
/// Non-reusable.
pub struct ImageAttempter<T> {
    /// Instance to which to bind the candidate-returning functions.
    /// For syntactic terseness when making lots of calls to, e.g., `TileSetStrings`.
    scope: T,
    /// The first valid filename tried if any, or the last filename tried if none have succeeded.
    last_tried_filename: Option<String>,
    /// Whether a valid image path has already been tried.
    /// Once this is true, no further calls to `try_image` have any effect.
    image_found: bool,
}

impl<T> ImageAttempter<T> {
    /// Creates a new ImageAttempter with the given scope.
    pub fn new(scope: T) -> Self {
        Self {
            scope,
            last_tried_filename: None,
            image_found: false,
        }
    }

    /// Chainable method that uses `ImageGetter::image_exists` to check whether an image exists.
    /// `get_path` and `get_image` will return either the first valid image passed here,
    /// or the last invalid image if none were valid. Calls after the first valid one are short-circuited.
    ///
    /// # Arguments
    ///
    /// * `file_name_fn` - Function that returns the filename of the image to check.
    ///   Bound to `scope`. Will not be run if a valid image has already been found.
    ///   May return `None` to skip this candidate entirely.
    ///
    /// # Returns
    ///
    /// Chainable `self` `ImageAttempter` extended by one check for `file_name_fn`
    pub fn try_image<F>(&mut self, file_name_fn: F) -> &mut Self
    where
        F: FnOnce(&T) -> Option<String>,
    {
        if !self.image_found {
            let image_path = file_name_fn(&self.scope);
            self.last_tried_filename = image_path.clone().or(self.last_tried_filename.clone());
            if let Some(path) = image_path {
                if ImageGetter::image_exists(&path) {
                    self.image_found = true;
                }
            }
        }
        self
    }

    /// Chainable method that makes multiple invocations to `try_image`.
    ///
    /// # Arguments
    ///
    /// * `file_names` - Any number of image candidate returning functions to pass to `try_image`.
    ///
    /// # Returns
    ///
    /// Chainable `self` `ImageAttempter` extended by zero or more checks for `file_names`
    pub fn try_images<F, I>(&mut self, file_names: I) -> &mut Self
    where
        F: FnOnce(&T) -> Option<String>,
        I: IntoIterator<Item = F>,
    {
        for file_name_fn in file_names {
            self.try_image(file_name_fn);
        }
        // *Could* skip calls/break loop if already image_found. But that means needing an internal guarantee/spec of try_image being same as no-op when image_found.
        self
    }

    /// Try to load era-specific image variants.
    ///
    /// Tries eras from the civ's current one down to the first era defined, by json order of eras.
    /// Result looks like "Plains-Rome-Ancient era": [style] goes before era if supplied.
    ///
    /// # Arguments
    ///
    /// * `civ_info` - The civ who owns the tile or unit, used for get_era_number and ruleset (but not for nation.get_style_or_civ_name)
    /// * `location_to_check` - The beginning of the filename to check
    /// * `style` - An optional string to load a civ- or style-specific sprite
    /// * `tile_set_strings` - The TileSetStrings instance to use for string formatting
    ///
    /// # Returns
    ///
    /// Chainable `self` `ImageAttempter` extended by one or more checks for era-specific images
    pub fn try_era_image(
        &mut self,
        civ_info: &Civilization,
        location_to_check: &str,
        style: Option<&str>,
        tile_set_strings: &TileSetStrings,
    ) -> &mut Self {
        let era_range: RangeInclusive<i32> = (0..=civ_info.get_era_number()).rev();

        let file_name_fns: Vec<Box<dyn FnOnce(&T) -> Option<String> + '_>> = era_range
            .map(|era_num| {
                let era = civ_info.game_info.ruleset.eras.keys().nth(era_num as usize).unwrap();
                let location_to_check = location_to_check.to_string();
                let style = style.map(|s| s.to_string());
                let tile_set_strings = tile_set_strings.clone();

                Box::new(move |_scope: &T| {
                    if let Some(style_str) = &style {
                        Some(tile_set_strings.get_string(
                            &location_to_check,
                            &tile_set_strings.tag,
                            style_str,
                            &tile_set_strings.tag,
                            era,
                        ))
                    } else {
                        Some(tile_set_strings.get_string(
                            &location_to_check,
                            &tile_set_strings.tag,
                            era,
                        ))
                    }
                }) as Box<dyn FnOnce(&T) -> Option<String>>
            })
            .collect();

        self.try_images(file_name_fns)
    }

    /// Returns the first valid image filename given to `try_image` if any,
    /// or the last tried image filename otherwise.
    pub fn get_path(&self) -> Option<&String> {
        self.last_tried_filename.as_ref()
    }

    /// Returns the first valid image filename given to `try_image` if any,
    /// or `None` if no valid image was tried.
    pub fn get_path_or_null(&self) -> Option<&String> {
        if self.image_found {
            self.last_tried_filename.as_ref()
        } else {
            None
        }
    }

    /// Returns the first valid image specified to `try_image` if any,
    /// or the last tried image otherwise.
    pub fn get_image(&self) -> Option<ggez::graphics::Image> {
        if let Some(path) = &self.last_tried_filename {
            Some(ImageGetter::get_image(path))
        } else {
            None
        }
    }
}