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

#[derive(Debug, PartialEq, Eq)]
pub struct CartHeader {
    pub title: String,
    pub cgb_flag: u8,
    pub new_licensee_code: String,
    pub sgb_flag: u8,
    pub cartridge_type: u8,
    pub rom_size: u8,
    pub ram_size: u8,
    pub destination_code: u8,
    pub old_licensee_code: u8,
    pub mask_rom_version: u8,
    pub header_checksum: u8,
    pub computed_header_checksum: u8,
    pub global_checksum: u16,
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
        let rom_size = rom[OFFSET_ROM_SIZE];
        let ram_size = rom[OFFSET_RAM_SIZE];
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
            cartridge_type,
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
            "Title: {}, CGB Flag: 0x{:02X}, Cartridge Type: 0x{:02X}, ROM Size: 0x{:02X}, RAM Size: 0x{:02X}",
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

#[derive(Debug)]
pub struct Cart {
    rom: Vec<u8>,
    pub header: CartHeader,
}

impl Cart {
    pub fn from_bytes(rom: Vec<u8>) -> Result<Cart, CartError> {
        let header = CartHeader::parse(&rom)?;
        Ok(Cart { rom, header })
    }

    pub fn read_rom(&self, addr: u16) -> u8 {
        self.rom[addr as usize]
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
