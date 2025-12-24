use boy_core::cart::Cart;
use boy_core::gameboy::GameBoy;
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

    let mut gameboy = GameBoy::new(cart);

    loop {
        gameboy.step();
    }
}
