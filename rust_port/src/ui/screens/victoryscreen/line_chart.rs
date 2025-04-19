// Source: orig_src/core/src/com/unciv/ui/screens/victoryscreen/LineChart.kt

use std::rc::Rc;
use std::collections::HashMap;
use egui::{Color32, Ui, Vec2, Response, Rect, Stroke, Align, RichText};
use crate::models::civilization::Civilization;
use crate::ui::images::ImageGetter;
use crate::utils::translation::tr;
use super::data_point::DataPoint;
use super::victory_screen_civ_group::VictoryScreenCivGroup;

/// A line chart for displaying civilization statistics over time
pub struct LineChart {
    /// The civilization viewing the chart
    viewing_civ: Rc<Civilization>,
    /// Width of axis lines
    axis_line_width: f32,
    /// Color of axis lines
    axis_color: Color32,
    /// Color of axis labels
    axis_label_color: Color32,
    /// Padding between axis and labels
    axis_to_label_padding: f32,
    /// Width of chart lines
    chart_line_width: f32,
    /// Width of orientation lines
    orientation_line_width: f32,
    /// Color of orientation lines
    orientation_line_color: Color32,
    /// Maximum number of labels
    max_labels: i32,
    /// X-axis labels
    x_labels: Vec<i32>,
    /// Y-axis labels
    y_labels: Vec<i32>,
    /// Data points for the chart
    data_points: Vec<DataPoint<i32>>,
    /// Selected civilization
    selected_civ: Rc<Civilization>,
    /// Chart bounds
    bounds: Rect,
}

impl LineChart {
    /// Creates a new line chart
    pub fn new(viewing_civ: Rc<Civilization>) -> Self {
        Self {
            viewing_civ,
            axis_line_width: 2.0,
            axis_color: Color32::WHITE,
            axis_label_color: Color32::WHITE,
            axis_to_label_padding: 5.0,
            chart_line_width: 3.0,
            orientation_line_width: 0.5,
            orientation_line_color: Color32::LIGHT_GRAY,
            max_labels: 10,
            x_labels: Vec::new(),
            y_labels: Vec::new(),
            data_points: Vec::new(),
            selected_civ: Rc::new(Civilization::new()),
            bounds: Rect::NOTHING,
        }
    }

    /// Gets the turn at a given x coordinate
    pub fn get_turn_at(&self, x: f32) -> Option<std::ops::Range<i32>> {
        if self.x_labels.is_empty() {
            return None;
        }

        let widest_y_label_width = 50.0; // Approximate width of widest label
        let lines_min_x = widest_y_label_width + self.axis_to_label_padding + self.axis_line_width;
        let lines_max_x = self.bounds.width() - 50.0; // Approximate width of last x label

        if lines_min_x >= lines_max_x {
            return Some(self.x_labels[0]..self.x_labels[self.x_labels.len() - 1] + 1);
        }

        let ratio = (x - lines_min_x) / (lines_max_x - lines_min_x);
        let turn = (self.lerp(self.x_labels[0] as f32, self.x_labels[self.x_labels.len() - 1] as f32, ratio) as i32).max(1);

        Some(
            self.get_prev_number_divisible_by_pow_of_ten(turn - 1)
            ..self.get_next_number_divisible_by_pow_of_ten(turn + 1)
        )
    }

    /// Updates the chart with new data
    pub fn update(&mut self, new_data: Vec<DataPoint<i32>>, new_selected_civ: Rc<Civilization>) {
        self.selected_civ = new_selected_civ;
        self.data_points = new_data;
        self.update_labels();
    }

    /// Updates the axis labels
    fn update_labels(&mut self) {
        self.x_labels = self.generate_labels(&self.data_points, false);
        self.y_labels = self.generate_labels(&self.data_points, true);
    }

    /// Generates labels for the chart
    fn generate_labels(&self, value: &[DataPoint<i32>], y_axis: bool) -> Vec<i32> {
        if value.is_empty() {
            return vec![0];
        }

        let min_label_value = if y_axis {
            self.get_prev_number_divisible_by_pow_of_ten(
                value.iter().map(|dp| dp.y).min().unwrap_or(0)
            )
        } else {
            self.get_prev_number_divisible_by_pow_of_ten(
                value.iter().map(|dp| dp.x).min().unwrap_or(0)
            )
        };

        let max_label_value = if y_axis {
            self.get_next_number_divisible_by_pow_of_ten(
                value.iter().map(|dp| dp.y).max().unwrap_or(0)
            )
        } else {
            self.get_next_number_divisible_by_pow_of_ten(
                value.iter().map(|dp| dp.x).max().unwrap_or(0)
            )
        };

        let mut step_size_positive = ((max_label_value as f32) / self.max_labels as f32).ceil() as i32;

        if min_label_value < 0 {
            let mut step_size_negative = ((-min_label_value as f32) / self.max_labels as f32).ceil() as i32;
            let max_step = step_size_positive.max(step_size_negative);
            let step_count_negative = (min_label_value / max_step).floor() as i32;
            step_size_negative = if (step_count_negative).abs() < 2 {
                min_label_value.abs()
            } else {
                max_step
            };
            let step_count_positive = (max_label_value / max_step).ceil() as i32;
            step_size_positive = if (step_count_positive).abs() < 2 {
                max_label_value.abs()
            } else {
                max_step
            };

            let mut result = Vec::new();
            for i in step_count_negative..0 {
                result.push(i * step_size_negative);
            }
            if max_label_value != 0 {
                for i in 0..step_count_positive + 1 {
                    result.push(i * step_size_positive);
                }
            } else {
                result.push(0);
            }
            result
        } else if max_label_value != 0 {
            if min_label_value < step_size_positive {
                (0..self.max_labels + 1).map(|i| i * step_size_positive).collect()
            } else {
                step_size_positive = ((max_label_value - min_label_value) as f32 / self.max_labels as f32).ceil() as i32;
                (0..self.max_labels + 1).map(|i| min_label_value + i * step_size_positive).collect()
            }
        } else {
            vec![0, 1]
        }
    }

    /// Gets the next number divisible by a power of 10
    fn get_next_number_divisible_by_pow_of_ten(&self, value: i32) -> i32 {
        if value == 0 {
            return 0;
        }
        let abs_value = value.abs();
        let number_of_digits = (abs_value as f32).log10().ceil() as i32;
        let one_with_zeros = 10.0_f32.powi(number_of_digits - 1);
        ((value as f32 / one_with_zeros).ceil() * one_with_zeros) as i32
    }

    /// Gets the previous number divisible by a power of 10
    fn get_prev_number_divisible_by_pow_of_ten(&self, value: i32) -> i32 {
        if value == 0 {
            return 0;
        }
        let abs_value = value.abs();
        let number_of_digits = (abs_value as f32).log10().ceil() as i32;
        let one_with_zeros = 10.0_f32.powi(number_of_digits - 1);
        ((value as f32 / one_with_zeros).floor() * one_with_zeros) as i32
    }

    /// Linear interpolation
    fn lerp(&self, a: f32, b: f32, t: f32) -> f32 {
        a + (b - a) * t
    }

    /// Draws the chart
    pub fn draw(&mut self, ui: &mut Ui) -> Response {
        let response = ui.allocate_response(ui.available_size(), egui::Sense::hover());
        self.bounds = response.rect;

        if self.x_labels.is_empty() || self.y_labels.is_empty() {
            return response;
        }

        let last_turn_data_points = self.get_last_turn_data_points();
        let label_height = 20.0; // Approximate height of a label
        let widest_y_label_width = 50.0; // Approximate width of widest label
        let y_axis_label_max_y = self.bounds.height() - label_height;
        let x_axis_labels_height = label_height;
        let zero_y_axis_label_height = label_height;
        let y_axis_label_min_y = x_axis_labels_height + self.axis_to_label_padding + self.axis_line_width / 2.0 - zero_y_axis_label_height / 2.0;
        let y_axis_label_y_range = y_axis_label_max_y - y_axis_label_min_y;

        // Draw y-axis labels
        let mut y_axis_y_position = 0.0;
        for (index, value) in self.y_labels.iter().enumerate() {
            let y_pos = y_axis_label_min_y + index as f32 * (y_axis_label_y_range / (self.y_labels.len() - 1) as f32);

            // Draw label
            ui.put(
                egui::pos2(self.bounds.left() + (widest_y_label_width - 40.0) / 2.0, self.bounds.top() + y_pos),
                egui::Label::new(RichText::new(tr(&value.to_string())).color(self.axis_label_color))
            );

            // Draw y-axis orientation lines and x-axis
            let zero_index = *value == 0;
            self.draw_line(
                ui,
                self.bounds.left() + widest_y_label_width + self.axis_to_label_padding + self.axis_line_width,
                self.bounds.top() + y_pos + label_height / 2.0,
                self.bounds.right(),
                self.bounds.top() + y_pos + label_height / 2.0,
                if zero_index { self.axis_color } else { self.orientation_line_color },
                if zero_index { self.axis_line_width } else { self.orientation_line_width }
            );

            if zero_index {
                y_axis_y_position = y_pos + label_height / 2.0;
            }
        }

        // Draw x-axis labels
        let last_x_axis_label_width = 50.0; // Approximate width of last x label
        let x_axis_label_min_x = self.bounds.left() + widest_y_label_width + self.axis_to_label_padding + self.axis_line_width / 2.0;
        let x_axis_label_max_x = self.bounds.right() - last_x_axis_label_width / 2.0;
        let x_axis_label_x_range = x_axis_label_max_x - x_axis_label_min_x;

        for (index, value) in self.x_labels.iter().enumerate() {
            let x_pos = x_axis_label_min_x + index as f32 * (x_axis_label_x_range / (self.x_labels.len() - 1) as f32);

            // Draw label
            ui.put(
                egui::pos2(x_pos - 20.0, self.bounds.top()),
                egui::Label::new(RichText::new(tr(&value.to_string())).color(self.axis_label_color))
            );

            // Draw x-axis orientation lines and y-axis
            self.draw_line(
                ui,
                x_pos,
                self.bounds.top() + label_height + self.axis_to_label_padding + self.axis_line_width,
                x_pos,
                self.bounds.bottom(),
                if index > 0 { self.orientation_line_color } else { self.axis_color },
                if index > 0 { self.orientation_line_width } else { self.axis_line_width }
            );
        }

        // Draw line charts for each civilization
        let lines_min_x = self.bounds.left() + widest_y_label_width + self.axis_to_label_padding + self.axis_line_width;
        let lines_max_x = self.bounds.right() - last_x_axis_label_width / 2.0;
        let lines_min_y = self.bounds.top() + y_axis_y_position;
        let lines_max_y = self.bounds.bottom() - label_height / 2.0;
        let scale_x = (lines_max_x - lines_min_x) / (self.x_labels.iter().max().unwrap() - self.x_labels.iter().min().unwrap()) as f32;
        let scale_y = (lines_max_y - lines_min_y) / (self.y_labels.iter().max().unwrap() - self.y_labels.iter().min().unwrap()) as f32;
        let negative_orientation_line_y_position = y_axis_label_min_y + label_height / 2.0;
        let min_x_label = *self.x_labels.iter().min().unwrap();
        let min_y_label = *self.y_labels.iter().min().unwrap();
        let negative_scale_y = if min_y_label < 0 {
            (negative_orientation_line_y_position - lines_min_y) / min_y_label as f32
        } else {
            1.0
        };

        // Sort points by x value
        let mut sorted_points = self.data_points.clone();
        sorted_points.sort_by(|a, b| a.x.cmp(&b.x));

        // Group points by civilization
        let mut points_by_civ: HashMap<Rc<Civilization>, Vec<DataPoint<i32>>> = HashMap::new();
        for point in sorted_points {
            points_by_civ.entry(point.civ.clone()).or_insert_with(Vec::new).push(point);
        }

        // Determine civilization iteration order
        let mut civ_iteration_order: Vec<Rc<Civilization>> = points_by_civ.keys().cloned().collect();
        civ_iteration_order.sort_by(|a, b| {
            let a_value = last_turn_data_points.get(a).map_or(0, |dp| dp.y);
            let b_value = last_turn_data_points.get(b).map_or(0, |dp| dp.y);
            a_value.cmp(&b_value)
        });

        // Move selected civilization to the end
        if let Some(pos) = civ_iteration_order.iter().position(|c| *c == self.selected_civ) {
            let civ = civ_iteration_order.remove(pos);
            civ_iteration_order.push(civ);
        }

        // Draw lines for each civilization
        for civ in civ_iteration_order {
            if let Some(points) = points_by_civ.get(&civ) {
                let scaled_points: Vec<DataPoint<f32>> = points.iter().map(|dp| {
                    if dp.y < 0 {
                        DataPoint::new(
                            lines_min_x + (dp.x - min_x_label) as f32 * scale_x,
                            lines_min_y + dp.y as f32 * negative_scale_y,
                            dp.civ.clone()
                        )
                    } else {
                        DataPoint::new(
                            lines_min_x + (dp.x - min_x_label) as f32 * scale_x,
                            lines_min_y + (dp.y - min_y_label) as f32 * scale_y,
                            dp.civ.clone()
                        )
                    }
                }).collect();

                // Simplify points using Douglas-Peucker algorithm
                let simplified_scaled_points = self.douglas_peucker(&scaled_points, 1.0);

                // Draw background line for selected civilization
                if civ == self.selected_civ {
                    for i in 1..simplified_scaled_points.len() {
                        let a = &simplified_scaled_points[i - 1];
                        let b = &simplified_scaled_points[i];
                        let selected_civ_background_color = if self.use_actual_color(&civ) {
                            civ.nation.get_inner_color()
                        } else {
                            Color32::LIGHT_GRAY
                        };
                        self.draw_line(
                            ui,
                            a.x, a.y, b.x, b.y,
                            selected_civ_background_color,
                            this.chart_line_width * 3.0
                        );
                    }
                }

                // Draw lines
                for i in 1..simplified_scaled_points.len() {
                    let a = &simplified_scaled_points[i - 1];
                    let b = &simplified_scaled_points[i];
                    let civ_line_color = if self.use_actual_color(&civ) {
                        civ.nation.get_outer_color()
                    } else {
                        Color32::DARK_GRAY
                    };
                    this.draw_line(ui, a.x, a.y, b.x, b.y, civ_line_color, this.chart_line_width);

                    // Draw the selected Civ icon on its last datapoint
                    if i == simplified_scaled_points.len() - 1 && self.selected_civ == civ && last_turn_data_points.contains_key(&civ) {
                        let (icon, _) = VictoryScreenCivGroup::get_civ_image_and_colors(&civ, &this.viewing_civ, VictoryScreenCivGroup::DefeatedPlayerStyle::Regular);
                        ui.put(
                            egui::pos2(b.x, b.y),
                            egui::Image::new(icon.texture_id(), egui::vec2(33.0, 33.0))
                                .tint(civ_line_color)
                        );
                    }
                }
            }
        }

        response
    }

    /// Checks if actual colors should be used for a civilization
    fn use_actual_color(&self, civ: &Rc<Civilization>) -> bool {
        self.viewing_civ.is_spectator() ||
            self.viewing_civ.is_defeated() ||
            self.viewing_civ.victory_manager.has_won() ||
            self.viewing_civ == *civ ||
            self.viewing_civ.knows(civ) ||
            civ.is_defeated()
    }

    /// Gets the last turn data points for each civilization
    fn get_last_turn_data_points(&self) -> HashMap<Rc<Civilization>, DataPoint<i32>> {
        let mut last_data_points = HashMap::new();
        for data_point in &this.data_points {
            if !last_data_points.contains_key(&data_point.civ) ||
               last_data_points[&data_point.civ].x < data_point.x {
                last_data_points.insert(data_point.civ.clone(), data_point.clone());
            }
        }
        last_data_points
    }

    /// Draws a line
    fn draw_line(&self, ui: &mut Ui, x1: f32, y1: f32, x2: f32, y2: f32, line_color: Color32, width: f32) {
        ui.painter().line_segment(
            [egui::pos2(x1, y1), egui::pos2(x2, y2)],
            Stroke::new(width, line_color)
        );
    }

    /// Simplifies a line using the Douglas-Peucker algorithm
    fn douglas_peucker(&self, points: &[DataPoint<f32>], epsilon: f32) -> Vec<DataPoint<f32>> {
        if points.len() < 3 {
            return points.to_vec();
        }

        let mut d_max = vec![0.0; points.len()];
        let mut index = 0;
        let mut max_distance = 0.0;

        // Find the point with the maximum distance from the line segment
        for i in 1..points.len() - 1 {
            let distance = self.perpendicular_distance(&points[i], &points[0], &points[points.len() - 1]);
            d_max[i] = distance;

            if distance > max_distance {
                index = i;
                max_distance = distance;
            }
        }

        // If the maximum distance is greater than epsilon, recursively simplify
        let mut result_list = Vec::new();
        if max_distance > epsilon {
            let recursive_list1 = self.douglas_peucker(&points[0..index + 1], epsilon);
            let recursive_list2 = self.douglas_peucker(&points[index..points.len()], epsilon);

            result_list.extend_from_slice(&recursive_list1[0..recursive_list1.len() - 1]);
            result_list.extend_from_slice(&recursive_list2);
        } else {
            result_list.push(points[0].clone());
            result_list.push(points[points.len() - 1].clone());
        }

        result_list
    }

    /// Calculates the perpendicular distance between a point and a line segment
    fn perpendicular_distance(&self, point: &DataPoint<f32>, start: &DataPoint<f32>, end: &DataPoint<f32>) -> f32 {
        let x = point.x;
        let y = point.y;
        let x1 = start.x;
        let y1 = start.y;
        let x2 = end.x;
        let y2 = end.y;

        let a = x - x1;
        let b = y - y1;
        let c = x2 - x1;
        let d = y2 - y1;

        let dot = a * c + b * d;
        let len_sq = c * c + d * d;
        let param = if len_sq == 0.0 { 0.0 } else { dot / len_sq };

        let xx = if param < 0.0 { x1 } else if param > 1.0 { x2 } else { x1 + param * c };
        let yy = if param < 0.0 { y1 } else if param > 1.0 { y2 } else { y1 + param * d };

        let dx = x - xx;
        let dy = y - yy;

        (dx * dx + dy * dy).sqrt()
    }
}