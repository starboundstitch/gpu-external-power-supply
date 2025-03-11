use embedded_hal::i2c::Error;

pub struct TPSC536C7<I> {
    address: u8,
    i2c: I,
}

pub enum Command {
    Page,
    Operation,
    OnOffConfig,
    ClearFaults,
    VOUTCommand,
    VOUTDroop,
    FrequencySwitch,
    IoutOCFaultLimit,
    StatusByte,
    ReadVout,
    ReadTemperature1,
    StatusAll,
    StatusExtended,
}

impl Command {
    pub fn to_address(self) -> u8 {
        match self {
            Command::Page => 0x00,
            Command::Operation => 0x01,
            Command::OnOffConfig => 0x02,
            Command::ClearFaults => 0x03,
            Command::VOUTCommand => 0x21,
            Command::VOUTDroop => 0x28,
            Command::FrequencySwitch => 0x33,
            Command::IoutOCFaultLimit => 0x46,
            Command::StatusByte => 0x78,
            Command::ReadVout => 0x8B,
            Command::ReadTemperature1 => 0x8D,
            Command::StatusAll => 0xDB,
            Command::StatusExtended => 0xDD,
        }
    }
}

pub enum Page {
    ChannelA,
    ChannelB,
    Both,
}

impl Page {
    pub fn to_bits(self) -> u8 {
        match self {
            Page::ChannelA => 0x00,
            Page::ChannelB => 0x01,
            Page::Both => 0xFF,
        }
    }
}

impl<I: embedded_hal::i2c::I2c> TPSC536C7<I> {
    pub fn new(i2c: I, address: u8) -> TPSC536C7<I> {
        let mut controller = TPSC536C7 { address, i2c };
        return controller;
    }

    pub fn command(&mut self, data: &[u8]) {
        match self.i2c.write(self.address, data) {
            Ok(_val) => defmt::trace!("Write_OK: {}", data),
            Err(val) => defmt::error!("Write Error: {}", val.kind()),
        }
    }

    pub fn on_off_config(&mut self) {
        self.command(&[Command::OnOffConfig.to_address(), 0x00]);
    }

    fn page(&mut self, ch: Page) {
        self.command(&[Command::Page.to_address(), ch.to_bits()]);
    }

    pub fn ch_a(&mut self) {
        self.page(Page::ChannelA);
    }

    pub fn ch_b(&mut self) {
        self.page(Page::ChannelB);
    }

    pub fn ch_ab(&mut self) {
        self.page(Page::Both);
    }

    pub fn clear_faults(&mut self) {
        self.command(&[Command::ClearFaults.to_address()]);
    }

}
