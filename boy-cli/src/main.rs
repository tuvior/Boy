use boy_core::cart::Cart;
use boy_core::gameboy::GameBoy;
use minifb::Window;
use minifb::WindowOptions;
use std::env;
use std::process;

fn main() {
    let mut args = env::args();
    let program = args.next().unwrap_or_else(|| "boy-cli".to_string());
    let rom_path = match args.next() {
        Some(path) => path,
        None => {
            eprintln!("usage: {program} <rom.gb>");
            process::exit(2);
        }
    };

    let rom = match std::fs::read(&rom_path) {
        Ok(data) => data,
        Err(err) => {
            eprintln!("failed to read rom {rom_path}: {err}");
            process::exit(1);
        }
    };

    let cart = match Cart::from_bytes(rom) {
        Ok(cart) => cart,
        Err(err) => {
            eprintln!("failed to parse rom header: {err}");
            process::exit(1);
        }
    };

    let title = &cart.header.title.clone();

    let mut gameboy = GameBoy::new(cart);

    const WIDTH: usize = 160;
    const HEIGHT: usize = 144;

    let mut window =
        Window::new(title, WIDTH, HEIGHT, WindowOptions::default()).unwrap_or_else(|e| {
            panic!("{}", e);
        });

    window.set_target_fps(60);

    while window.is_open() {
        let fb = gameboy.frame();

        window.update_with_buffer(&fb, WIDTH, HEIGHT).unwrap();
    }
}
