use core::cart::Cart;
use core::gameboy::GameBoy;
use core::gameboy::KeyStates;
use minifb::Key;
use minifb::Window;
use minifb::WindowOptions;
use std::env;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::Path;
use std::process;

fn main() {
    let mut args = env::args();
    let program = args.next().unwrap_or_else(|| "cli".to_string());
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

    let save_data = load_save_file(&rom_path);

    let cart = match Cart::from_bytes(rom, save_data) {
        Ok(cart) => cart,
        Err(err) => {
            eprintln!("failed to parse rom header: {err}");
            process::exit(1);
        }
    };

    let title = cart.get_title();
    let mut gameboy = GameBoy::new(cart);

    const WIDTH: usize = 160;
    const HEIGHT: usize = 144;

    let opts = WindowOptions {
        scale: minifb::Scale::X2,
        ..Default::default()
    };

    let mut window = Window::new(&title, WIDTH, HEIGHT, opts).unwrap_or_else(|e| {
        panic!("{}", e);
    });

    window.set_target_fps(60);

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let keys = build_key_state(&window.get_keys());
        gameboy.run_frame(keys);

        let fb = gameboy.get_last_frame_buffer();

        if window.is_key_pressed(Key::S, minifb::KeyRepeat::No) {
            dump_framebuffer_ppm("screenshot.ppm", &fb).unwrap();
        }

        window.update_with_buffer(&fb, WIDTH, HEIGHT).unwrap();
    }

    if let Some(save_data) = gameboy.save() {
        save_to_file(save_data, &rom_path).expect("Failed to created save file");
    }
}

pub fn build_save_path(rom_path: &str) -> String {
    let name = rom_path.rsplit_once(".").unwrap().0;
    format!("{name}.sav")
}

pub fn load_save_file(rom_path: &str) -> Option<Vec<u8>> {
    std::fs::read(build_save_path(rom_path)).ok()
}

pub fn save_to_file(data: Vec<u8>, rom_path: &str) -> std::io::Result<()> {
    let mut file = File::create(build_save_path(rom_path))?;
    file.write_all(&data)?;
    file.flush()?;

    Ok(())
}

pub fn dump_framebuffer_ppm<P: AsRef<Path>>(path: P, fb: &[u32; 160 * 144]) -> std::io::Result<()> {
    let file = File::create(path)?;
    let mut w = BufWriter::new(file);

    // P6 = binary RGB
    writeln!(w, "P6")?;
    writeln!(w, "160 144")?;
    writeln!(w, "255")?;

    for &color in fb.iter() {
        let r = ((color >> 16) & 0xFF) as u8;
        let g = ((color >> 8) & 0xFF) as u8;
        let b = (color & 0xFF) as u8;
        w.write_all(&[r, g, b])?;
    }

    w.flush()?;
    Ok(())
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
