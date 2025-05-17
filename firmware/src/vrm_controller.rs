use embedded_hal::i2c::Error;
use pmbus_types_rs::{slinear11, ulinear16};

pub struct TPSC536C7<I> {
    address: u8,
    i2c: I,
}

/// An abstracted way to generate i2c PMBUS Read Commands.

/// Self, Name, Command (u8), length (int), format (slinear or ulinear)
macro_rules! send_read {
    ($self:ident, $name:literal, $cmd:expr, $length:expr,  $format:ident) => {{
        let mut buf = [b'\0'; $length];
        match $self.i2c.write_read($self.address, &[$cmd], &mut buf) {
            Ok(_val) => defmt::trace!("{}_Read: {:#X}, {:#X}", $name, $cmd, buf),
            Err(val) => defmt::error!("{}_Read_Error: {:#X}, {}", $name, $cmd, val.kind()),
        }
        $format::to(to_u16(buf))
    }};
}

/// An abstracted way to generate i2c PMBUS Write Commands.

/// Self, Name, Command (u8), length (int), format (slinear or ulinear), data (float)
macro_rules! send_write {
    ($self:ident, $name:literal, $cmd:expr, $length:expr,  $format:ident, $data:expr) => {{
        let con = $format::from($data).to_be_bytes();

        let buf: &mut [u8] = &mut [0u8; $length + 1];
        buf[0..$length].copy_from_slice(&con[0..($length)]);
        buf[$length] = $cmd;
        buf.reverse();
        match $self.i2c.write($self.address, buf) {
            Ok(_val) => defmt::trace!("{}_Write: {}", $name, buf),
            Err(val) => defmt::error!("{}_Write_Error: {}", $name, val.kind()),
        }
    }};
}

/// An abstracted way to generate i2c PMBUS Commands.

/// Type (name but different) Name, Command (u8), length (int), format (slinear or ulinear)
macro_rules! build_command {
    ($type:ident, $name:literal, $cmd:expr, $length:expr,  $format:ident) => {
        pub struct $type<'a, I> {
            dev: &'a mut TPSC536C7<I>,
        }

        impl<'a, I: embedded_hal::i2c::I2c> $type<'a, I> {
            pub fn read(&mut self) -> f32 {
                let dev = &mut self.dev;
                send_read!(dev, $name, $cmd, $length, $format)
            }
            pub fn write(&mut self, val: f32) {
                let dev = &mut self.dev;
                send_write!(dev, $name, $cmd, $length, $format, val)
            }
        }
    };
}

build_command!(
    VOUTCommand,
    "VOutCommand",
    Command::VOUTCommand.to_address(),
    2,
    ulinear16
);
build_command!(
    VOUTMax,
    "VOutMax",
    Command::VOUTMax.to_address(),
    2,
    ulinear16
);
build_command!(
    VOUTMin,
    "VOutMin",
    Command::VOUTMin.to_address(),
    2,
    ulinear16
);
build_command!(
    IOUTOCFaultLimit,
    "IOutOCFaultLimit",
    Command::IoutOCFaultLimit.to_address(),
    2,
    slinear11
);

pub enum Command {
    Page,
    Operation,
    OnOffConfig,
    ClearFaults,
    VOUTCommand,
    VOUTMax,
    VOUTDroop,
    VOUTMin,
    FrequencySwitch,
    IoutOCFaultLimit,
    StatusByte,
    ReadIout,
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
            Command::VOUTMax => 0x24,
            Command::VOUTDroop => 0x28,
            Command::VOUTMin => 0x2B,
            Command::FrequencySwitch => 0x33,
            Command::IoutOCFaultLimit => 0x46,
            Command::StatusByte => 0x78,
            Command::ReadVout => 0x8B,
            Command::ReadIout => 0x8C,
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
        let controller = TPSC536C7 { address, i2c };
        return controller;
    }

    pub fn command(&mut self, data: &[u8]) {
        match self.i2c.write(self.address, data) {
            Ok(_val) => defmt::trace!("Write_OK: {}", data),
            Err(val) => defmt::error!("Write Error: {}", val.kind()),
        }
    }

    pub fn read(&mut self, cmd: u8, buf: &mut [u8]) {
        match self.i2c.write_read(self.address, &[cmd], buf) {
            Ok(_val) => defmt::trace!("Read_OK: {:#X}, {:#X}", cmd, buf),
            Err(val) => defmt::error!("Controller Read: {:#X}, {}", cmd, val.kind()),
        }
    }

    pub fn on_off_config(&mut self, val: u8) {
        self.command(&[Command::OnOffConfig.to_address(), val]);
    }

    // PAGING OPTIONS
    fn page(&mut self, ch: Page) -> &mut Self {
        self.command(&[Command::Page.to_address(), ch.to_bits()]);
        self
    }

    pub fn ch_a(&mut self) -> &mut Self {
        self.page(Page::ChannelA)
    }

    pub fn ch_b(&mut self) -> &mut Self {
        self.page(Page::ChannelB)
    }

    pub fn ch_ab(&mut self) -> &mut Self {
        self.page(Page::Both)
    }

    pub fn clear_faults(&mut self) {
        self.command(&[Command::ClearFaults.to_address()]);
    }

    pub fn status_byte(&mut self) {
        let mut buf = [b'\0'; 1];
        self.read(Command::StatusByte.to_address(), &mut buf);
    }

    pub fn read_page(&mut self) {
        let mut buf = [b'\0'; 1];
        self.read(Command::Page.to_address(), &mut buf);
    }

    pub fn read_status_extended(&mut self) {
        let mut buf = [b'\0'; 7];
        self.read(Command::StatusExtended.to_address(), &mut buf);
    }

    pub fn read_status_all(&mut self) {
        let mut buf = [b'\0'; 18];
        self.read(Command::StatusAll.to_address(), &mut buf);
    }

    // READ WRITE COMMANDS
    /// Reads / Writes to the voltage output setpoint for the paged channel
    pub fn vout_command(&mut self) -> VOUTCommand<I> {
        VOUTCommand { dev: self }
    }

    /// Reads / Writes to the voltage max for the paged channel
    pub fn vout_max(&mut self) -> VOUTMax<I> {
        VOUTMax { dev: self }
    }

    /// Reads / Writes to the voltage min for the paged channel
    pub fn vout_min(&mut self) -> VOUTMin<I> {
        VOUTMin { dev: self }
    }

    /// Reads / Writes to the current output setpoint for the paged channel
    ///
    /// Is phased (can read the individual phases and set individual phase)
    /// values if we want to implement that
    pub fn iout_oc_fault_limit(&mut self) -> IOUTOCFaultLimit<I> {
        IOUTOCFaultLimit { dev: self }
    }

    // READ ONLY COMMANDS
    /// Reads the ouput voltage at the paged channel
    pub fn read_vout(&mut self) -> f32 {
        send_read!(
            self,
            "ReadVout",
            Command::ReadVout.to_address(),
            2,
            ulinear16
        )
    }
    /// Reads the output current at the paged channel
    pub fn read_iout(&mut self) -> f32 {
        send_read!(
            self,
            "ReadIout",
            Command::ReadIout.to_address(),
            2,
            slinear11
        )
    }
    /// Reads the output temperature at the paged channel
    pub fn read_temperature_1(&mut self) -> f32 {
        send_read!(
            self,
            "READTemperature1",
            Command::ReadTemperature1.to_address(),
            2,
            slinear11
        )
    }
}

pub fn to_u16(val: [u8; 2]) -> u16 {
    (val[1] as u16) << 8 | val[0] as u16
}
