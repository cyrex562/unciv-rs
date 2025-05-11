# Unciv Rust Port

This is a Rust port of the [Unciv](https://github.com/yairm210/Unciv) game, which is a free and open-source turn-based strategy game based on Civilization V.

## Project Structure

The project is organized as follows:

- `src/ui/` - UI components and screens
  - `components/` - Reusable UI components
  - `images/` - Image loading and manipulation utilities
  - `popups/` - Popup dialogs
  - `screens/` - Game screens

## Dependencies

- `eframe` - Framework for creating native applications with egui
- `egui` - Immediate mode GUI library
- `lazy_static` - Macro for creating static variables
- `log` - Logging facade
- `env_logger` - Logging implementation

## Building and Running

To build and run the project:

```bash
cargo build
cargo run
```

## Conversion Notes

This project is a port of the original Kotlin-based Unciv game to Rust. The conversion process involves:

1. Analyzing the original Kotlin code
2. Designing equivalent Rust structures and traits
3. Implementing the functionality in Rust
4. Integrating with Rust-specific libraries and frameworks

## TODO

- [ ] Complete the port of all UI components
- [ ] Implement game logic
- [ ] Add unit tests
- [ ] Improve performance
- [ ] Add documentation