use embedded_graphics::prelude::Point;

#[derive(Default)]
pub struct Navigation {
    // x, y
    position: (i32, i32),
    mode: Mode,
}

// Implements navigation across the microcontroller for the user input
impl Navigation {
    const X_MAX: i32 = 2;
    const Y_MAX: i32 = 3;

    pub fn get_position(&self) -> (i32, i32) {
        self.position
    }

    pub fn get_point(&self) -> Point {
        translate_point(self.position)
    }

    pub fn move_left(&mut self) {
        if self.position.0 > 0 {
            self.position.0 -= 1;
        }
    }

    pub fn move_right(&mut self) {
        if self.position.0 < Self::X_MAX - 1 {
            self.position.0 += 1;
        }
    }

    pub fn move_up(&mut self) {
        if self.position.1 > 0 {
            self.position.1 -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.position.1 < Self::Y_MAX - 1 {
            self.position.1 += 1;
        }
    }

    pub fn change_mode(&mut self) {
        match self.mode {
            Mode::Navigation => self.mode = Mode::Update,
            Mode::Update => self.mode = Mode::Navigation,
        }
    }

    pub fn get_mode(&self) -> &Mode {
        &self.mode
    }
}

pub enum Mode {
    Navigation,
    Update,
}

impl Default for Mode {
    fn default() -> Mode {
        Mode::Navigation
    }
}

#[derive(Default)]
pub struct Channel {
    voltage: f32,
    set_voltage: f32,
    current: f32,
    current_limit: f32,
    temperature: f32,
}

impl Channel {
    pub fn voltage(&self) -> f32 {
        self.voltage
    }
    pub fn set_voltage(&mut self, val: f32) {
        // Change once I2C hooked up to be self.set_voltage
        self.voltage = val;
    }
    pub fn current(&self) -> f32 {
        self.current
    }
    pub fn set_current(&mut self, val: f32) {
        // Change once I2C hooked up to be self.current_limit
        self.current = val;
    }
    pub fn temperature(&self) -> f32 {
        self.temperature
    }
}

pub fn translate_point(point: (i32, i32)) -> Point {
    Point::new(27 + 6 * 9 * point.0, 16 + 16 * point.1)
}
