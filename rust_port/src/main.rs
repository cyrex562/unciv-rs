use std::ffi::c_void;
use libc::{c_int, c_uint, c_char};

mod serializers;
mod json;
mod automation;
mod models;

#[link(name = "SDL3")]
extern "C" {
    fn SDL_Init(flags: c_uint) -> c_int;
    fn SDL_Quit();
    fn SDL_CreateWindow(title: *const c_char, x: c_int, y: c_int, w: c_int, h: c_int, flags: c_uint) -> *mut c_void;
    fn SDL_DestroyWindow(window: *mut c_void);
    fn SDL_PollEvent(event: *mut c_void) -> c_int;
    fn SDL_Delay(ms: c_uint);
}

const SDL_INIT_VIDEO: c_uint = 0x00000020;
const SDL_WINDOWPOS_CENTERED: c_int = 0x2FFF0000;
const SDL_WINDOW_SHOWN: c_uint = 0x00000004;

#[repr(C)]
struct SDL_Event {
    _type: c_uint,
    _padding: [c_char; 56],
}

const SDL_QUIT: c_uint = 0x100;

fn main() -> Result<(), String> {
    unsafe {
        if SDL_Init(SDL_INIT_VIDEO) != 0 {
            return Err("Failed to initialize SDL".to_string());
        }

        let title = std::ffi::CString::new("Rust Port").unwrap();
        let window = SDL_CreateWindow(
            title.as_ptr(),
            SDL_WINDOWPOS_CENTERED,
            SDL_WINDOWPOS_CENTERED,
            800,
            600,
            SDL_WINDOW_SHOWN,
        );

        if window.is_null() {
            SDL_Quit();
            return Err("Failed to create window".to_string());
        }

        let mut event = SDL_Event {
            _type: 0,
            _padding: [0; 56],
        };

        'running: loop {
            while SDL_PollEvent(&mut event as *mut _ as *mut c_void) != 0 {
                if event._type == SDL_QUIT {
                    break 'running;
                }
            }

            SDL_Delay(16); // ~60 FPS
        }

        SDL_DestroyWindow(window);
        SDL_Quit();
    }

    Ok(())
}
