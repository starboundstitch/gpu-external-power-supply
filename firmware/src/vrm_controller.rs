use embedded_hal::i2c::Error;

pub struct TPSC536C7<I> {
    address: u8,
    i2c: I,
}

pub enum Command {
    OnOffConfig,
impl Command {
    pub fn to_address(self) -> u8 {
        match self {
            Command::OnOffConfig => 0x02,
        }
    }
}

}
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

