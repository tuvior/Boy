use std::env;
use std::process;
use boy_core::cart::Cart;
use boy_core::mmu::MMU;

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

    let header = &cart.header;
    println!("Loaded ROM: {header}");

    let mmu = MMU::new(cart);

    let entry = mmu.rb(0x0100);
    println!("Entry: {entry}");

    let title = mmu.rb(0x0134);
    println!("Title: {title}");
}
