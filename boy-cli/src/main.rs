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

    let cart = match boy_core::cart::Cart::from_bytes(rom) {
        Ok(cart) => cart,
        Err(err) => {
            eprintln!("failed to parse rom header: {err}");
            process::exit(1);
        }
    };

    let header = &cart.header;
    println!("title: {}", header.title);
    println!("cgb_flag: 0x{:02X}", header.cgb_flag);
    println!("new_licensee_code: {}", header.new_licensee_code);
    println!("sgb_flag: 0x{:02X}", header.sgb_flag);
    println!("cartridge_type: 0x{:02X}", header.cartridge_type);
    println!("rom_size: 0x{:02X}", header.rom_size);
    println!("ram_size: 0x{:02X}", header.ram_size);
    println!("destination_code: 0x{:02X}", header.destination_code);
    println!("old_licensee_code: 0x{:02X}", header.old_licensee_code);
    println!("mask_rom_version: 0x{:02X}", header.mask_rom_version);
    println!("header_checksum: 0x{:02X}", header.header_checksum);
    println!(
        "computed_header_checksum: 0x{:02X}",
        header.computed_header_checksum
    );
    println!("global_checksum: 0x{:04X}", header.global_checksum);
}
