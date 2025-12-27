use boy_core::cart::Cart;
use boy_core::gameboy::GameBoy;
use boy_core::gameboy::KeyStates;
use minifb::Key;
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
        let keys = build_key_state(&window.get_keys());
        let fb = gameboy.frame(keys);

        window.update_with_buffer(&fb, WIDTH, HEIGHT).unwrap();
    }
}

fn build_key_state(keys: &[Key]) -> KeyStates {
    KeyStates {
        a: keys.contains(&Key::Z),
        b: keys.contains(&Key::X),
        start: keys.contains(&Key::Enter),
        select: keys.contains(&Key::RightShift),
        up: keys.contains(&Key::Up),
        down: keys.contains(&Key::Down),
        left: keys.contains(&Key::Left),
        right: keys.contains(&Key::Right),
    }
}
