pub struct RTC {
    startup: u64,
    rtc_s: u8,  // $08 	Seconds	0-59 ($00-$3B)
    rtc_m: u8,  // $09	Minutes	0-59 ($00-$3B)
    rtc_h: u8,  // $0A	Hours	0-23 ($00-$17)
    rtc_dl: u8, // $0B	Lower 8 bits of Day Counter	($00-$FF)
    rtc_dh: u8, // $0C
    latch: bool,
}

#[derive(Debug)]
pub struct Counters {
    seconds: u8,
    minutes: u8,
    hours: u8,
    days: u64,
}

impl RTC {
    pub fn init() -> RTC {
        let path =
            std::path::PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
                .join(".boy/rtc_startup");

        let startup = if path.exists() {
            std::fs::read_to_string(&path)
                .unwrap()
                .trim()
                .parse()
                .unwrap()
        } else {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();

            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, timestamp.to_string()).unwrap();

            timestamp
        };

        Self {
            startup,
            rtc_s: 0,
            rtc_m: 0,
            rtc_h: 0,
            rtc_dl: 0,
            rtc_dh: 0,
            latch: false,
        }
    }

    pub fn latch(&mut self, value: u8) {
        if value == 0x00 {
            self.latch = true;
        } else if value == 0x01 && self.latch {
            self.latch = false;
            self.latch_values();
        } else {
            self.latch = false;
        }
    }

    fn latch_values(&mut self) {
        let counters = self.get_counters();

        self.rtc_s = counters.seconds;
        self.rtc_m = counters.minutes;
        self.rtc_h = counters.hours;
        self.rtc_dl = (counters.days & 0xFF) as u8;
        self.rtc_dh = ((counters.days >> 8) & 0xFF) as u8;
    }

    pub fn write_regisetr(&mut self, register: u8, value: u8) {
        match register {
            0x08 => self.rtc_s = value,
            0x09 => self.rtc_m = value,
            0x0A => self.rtc_h = value,
            0x0B => self.rtc_dl = value,
            0x0C => self.rtc_dh = value,
            _ => unreachable!(),
        }
    }

    pub fn read_register(&self, register: u8) -> u8 {
        match register {
            0x08 => self.rtc_s,
            0x09 => self.rtc_m,
            0x0A => self.rtc_h,
            0x0B => self.rtc_dl,
            0x0C => self.rtc_dh,
            _ => unreachable!(),
        }
    }

    pub fn get_counters(&self) -> Counters {
        let startup = std::time::UNIX_EPOCH + std::time::Duration::from_secs(self.startup);
        let duration = std::time::SystemTime::now()
            .duration_since(startup)
            .unwrap();

        let days = duration.as_secs() / 86400;
        let hours = (duration.as_secs() % 86400) / 3600;
        let minutes = (duration.as_secs() % 3600) / 60;
        let seconds = duration.as_secs() % 60;

        Counters {
            seconds: seconds as u8,
            minutes: minutes as u8,
            hours: hours as u8,
            days,
        }
    }
}
