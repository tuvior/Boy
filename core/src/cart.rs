use crate::mbc::{
    MemoryController, Missing, mbc1::Mbc1, mbc2::Mbc2, mbc3::Mbc3, rom_only::RomOnly,
};

const HEADER_END: usize = 0x14F;
const OFFSET_TITLE_START: usize = 0x134;
const OFFSET_TITLE_END: usize = 0x143;
const OFFSET_CGB_FLAG: usize = 0x143;
const OFFSET_LICENSEE_NEW_START: usize = 0x144;
const OFFSET_LICENSEE_NEW_END: usize = 0x145;
const OFFSET_SGB_FLAG: usize = 0x146;
const OFFSET_CARTRIDGE_TYPE: usize = 0x147;
const OFFSET_ROM_SIZE: usize = 0x148;
const OFFSET_RAM_SIZE: usize = 0x149;
const OFFSET_DESTINATION_CODE: usize = 0x14A;
const OFFSET_LICENSEE_OLD: usize = 0x14B;
const OFFSET_MASK_ROM_VERSION: usize = 0x14C;
const OFFSET_HEADER_CHECKSUM: usize = 0x14D;
const OFFSET_GLOBAL_CHECKSUM_START: usize = 0x14E;
const OFFSET_GLOBAL_CHECKSUM_END: usize = 0x14F;

#[allow(unused)]
pub struct CartHeader {
    title: String,
    cgb_flag: u8,
    new_licensee_code: String,
    sgb_flag: u8,
    cartridge_type: CartridgeType,
    rom_size: u32,
    ram_size: u32,
    destination_code: u8,
    old_licensee_code: u8,
    mask_rom_version: u8,
    header_checksum: u8,
    computed_header_checksum: u8,
    global_checksum: u16,
}

fn rom_size_from_id(id: u8) -> u32 {
    match id {
        0x00 => 32 * 1024,
        0x01 => 64 * 1024,
        0x02 => 128 * 1024,
        0x03 => 256 * 1024,
        0x04 => 512 * 1024,
        0x05 => 1024 * 1024,
        0x06 => 2 * 1024 * 1024,
        0x07 => 4 * 1024 * 1024,
        0x08 => 8 * 1024 * 1024,
        _ => unreachable!(),
    }
}

fn ram_size_from_id(id: u8) -> u32 {
    match id {
        0x00 => 0,
        0x01 => panic!("Should be unused"),
        0x02 => 8 * 1024,
        0x03 => 32 * 1024,
        0x04 => 128 * 1024,
        0x05 => 64 * 1024,
        _ => unreachable!(),
    }
}

impl CartHeader {
    fn parse(rom: &[u8]) -> Result<CartHeader, CartError> {
        if rom.len() <= HEADER_END {
            return Err(CartError::RomTooSmall { len: rom.len() });
        }

        let title_bytes = &rom[OFFSET_TITLE_START..=OFFSET_TITLE_END];
        let title = ascii_from_bytes(title_bytes);

        let new_licensee_bytes = &rom[OFFSET_LICENSEE_NEW_START..=OFFSET_LICENSEE_NEW_END];
        let new_licensee_code = ascii_from_bytes(new_licensee_bytes);

        let cgb_flag = rom[OFFSET_CGB_FLAG];
        let sgb_flag = rom[OFFSET_SGB_FLAG];
        let cartridge_type = rom[OFFSET_CARTRIDGE_TYPE];
        let rom_size = rom_size_from_id(rom[OFFSET_ROM_SIZE]);
        let ram_size = ram_size_from_id(rom[OFFSET_RAM_SIZE]);
        let destination_code = rom[OFFSET_DESTINATION_CODE];
        let old_licensee_code = rom[OFFSET_LICENSEE_OLD];
        let mask_rom_version = rom[OFFSET_MASK_ROM_VERSION];
        let header_checksum = rom[OFFSET_HEADER_CHECKSUM];
        let computed_header_checksum = compute_header_checksum(rom);
        let global_checksum = u16::from_be_bytes([
            rom[OFFSET_GLOBAL_CHECKSUM_START],
            rom[OFFSET_GLOBAL_CHECKSUM_END],
        ]);

        Ok(CartHeader {
            title,
            cgb_flag,
            new_licensee_code,
            sgb_flag,
            cartridge_type: CartridgeType::from_code(cartridge_type),
            rom_size,
            ram_size,
            destination_code,
            old_licensee_code,
            mask_rom_version,
            header_checksum,
            computed_header_checksum,
            global_checksum,
        })
    }
}

impl std::fmt::Display for CartHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Title: {}, CGB Flag: 0x{:02X}, Cartridge Type: {:?}, ROM Size: 0x{:02X}, RAM Size: 0x{:02X}",
            self.title, self.cgb_flag, self.cartridge_type, self.rom_size, self.ram_size
        )
    }
}

#[derive(Debug)]
pub enum CartError {
    RomTooSmall { len: usize },
}

impl std::fmt::Display for CartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CartError::RomTooSmall { len } => {
                write!(f, "rom is too small for header: {len} bytes")
            }
        }
    }
}

impl std::error::Error for CartError {}

pub struct Cart {
    pub header: CartHeader,
    pub controller: Box<dyn MemoryController>,
}

impl Cart {
    pub fn from_bytes(rom: Vec<u8>, save_data: Option<Vec<u8>>) -> Result<Cart, CartError> {
        let header = CartHeader::parse(&rom)?;

        let controller: Box<dyn MemoryController> = match header.cartridge_type {
            CartridgeType::RomOnly => Box::new(RomOnly::new(rom, header.ram_size)),
            CartridgeType::Mbc1 {
                has_ram,
                has_battery,
            } => Box::new(Mbc1::new(
                rom,
                header.ram_size,
                has_ram,
                has_battery,
                save_data,
            )),
            CartridgeType::Mbc2 { has_battery } => Box::new(Mbc2::new(rom, has_battery, save_data)),
            CartridgeType::Mbc3 {
                has_timer,
                has_ram,
                has_battery,
            } => Box::new(Mbc3::new(
                rom,
                header.ram_size,
                has_ram,
                has_battery,
                has_timer,
                save_data,
            )),
            _ => Box::new(Missing),
        };

        Ok(Cart { header, controller })
    }

    pub fn rb(&mut self, addr: u16) -> u8 {
        self.controller.rb(addr)
    }

    pub fn wb(&mut self, addr: u16, value: u8) {
        self.controller.wb(addr, value)
    }

    pub fn save(&self) -> Option<Vec<u8>> {
        self.controller.save()
    }

    pub fn get_title(&self) -> String {
        self.header.title.clone()
    }
}

fn compute_header_checksum(rom: &[u8]) -> u8 {
    let header_bytes = &rom[OFFSET_TITLE_START..=OFFSET_MASK_ROM_VERSION];
    header_bytes
        .iter()
        .fold(0, |c, &b| c.wrapping_sub(b).wrapping_sub(1))
}

fn ascii_from_bytes(bytes: &[u8]) -> String {
    let term = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..term]).to_string()
}

#[derive(Debug)]
pub enum CartridgeType {
    RomOnly,
    Mbc1 {
        has_ram: bool,
        has_battery: bool,
    },
    Mbc2 {
        has_battery: bool,
    },
    Mbc3 {
        has_timer: bool,
        has_ram: bool,
        has_battery: bool,
    },
    Mbc5 {
        has_ram: bool,
        has_battery: bool,
        has_rumble: bool,
    },
    Mbc6,
    Mbc7 {
        has_sensor: bool,
        has_rumble: bool,
        has_ram: bool,
        has_battery: bool,
    },
    RomRam {
        has_battery: bool,
    },
    Mmm01 {
        has_ram: bool,
        has_battery: bool,
    },
    PocketCamera,
    BandaiTama5,
    HuC3,
    HuC1 {
        has_ram: bool,
        has_battery: bool,
    },
}

impl CartridgeType {
    pub fn from_code(code: u8) -> Self {
        match code {
            0x00 => CartridgeType::RomOnly,
            0x01 => CartridgeType::Mbc1 {
                has_ram: false,
                has_battery: false,
            },
            0x02 => CartridgeType::Mbc1 {
                has_ram: true,
                has_battery: false,
            },
            0x03 => CartridgeType::Mbc1 {
                has_ram: true,
                has_battery: true,
            },
            0x05 => CartridgeType::Mbc2 { has_battery: false },
            0x06 => CartridgeType::Mbc2 { has_battery: true },
            0x08 => CartridgeType::RomRam { has_battery: false },
            0x09 => CartridgeType::RomRam { has_battery: true },
            0x0B => CartridgeType::Mmm01 {
                has_ram: false,
                has_battery: false,
            },
            0x0C => CartridgeType::Mmm01 {
                has_ram: true,
                has_battery: false,
            },
            0x0D => CartridgeType::Mmm01 {
                has_ram: true,
                has_battery: true,
            },
            0x0F => CartridgeType::Mbc3 {
                has_timer: true,
                has_ram: false,
                has_battery: true,
            },
            0x10 => CartridgeType::Mbc3 {
                has_timer: true,
                has_ram: true,
                has_battery: true,
            },
            0x11 => CartridgeType::Mbc3 {
                has_timer: false,
                has_ram: false,
                has_battery: false,
            },
            0x12 => CartridgeType::Mbc3 {
                has_timer: false,
                has_ram: true,
                has_battery: false,
            },
            0x13 => CartridgeType::Mbc3 {
                has_timer: false,
                has_ram: true,
                has_battery: true,
            },
            0x19 => CartridgeType::Mbc5 {
                has_ram: false,
                has_battery: false,
                has_rumble: false,
            },
            0x1A => CartridgeType::Mbc5 {
                has_ram: true,
                has_battery: false,
                has_rumble: false,
            },
            0x1B => CartridgeType::Mbc5 {
                has_ram: true,
                has_battery: true,
                has_rumble: false,
            },
            0x1C => CartridgeType::Mbc5 {
                has_ram: false,
                has_battery: false,
                has_rumble: true,
            },
            0x1D => CartridgeType::Mbc5 {
                has_ram: true,
                has_battery: false,
                has_rumble: true,
            },
            0x1E => CartridgeType::Mbc5 {
                has_ram: true,
                has_battery: true,
                has_rumble: true,
            },
            0x20 => CartridgeType::Mbc6,
            0x22 => CartridgeType::Mbc7 {
                has_sensor: true,
                has_rumble: true,
                has_ram: true,
                has_battery: true,
            },
            0xFC => CartridgeType::PocketCamera,
            0xFD => CartridgeType::BandaiTama5,
            0xFE => CartridgeType::HuC3,
            0xFF => CartridgeType::HuC1 {
                has_ram: true,
                has_battery: true,
            },
            _ => unreachable!(),
        }
    }
}
