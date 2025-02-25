use embedded_hal::i2c::Error;

pub struct TPSC536C7<I> {
    address: u8,
    i2c: I,
}

pub enum Command {
    Page,
    OnOffConfig,
impl Command {
    pub fn to_address(self) -> u8 {
        match self {
            Command::Page => 0x00,
            Command::OnOffConfig => 0x02,
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
        controller.on_off_config();
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
