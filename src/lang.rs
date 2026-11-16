#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Lang {
    Ru,
    En,
}

impl Lang {
    pub fn toggle(&mut self) {
        *self = if *self == Lang::Ru {
            Lang::En
        } else {
            Lang::Ru
        };
    }

    #[cfg(target_os = "linux")]
    pub fn from_group(group: u32) -> Self {
        if group == 1 {
            Lang::Ru
        } else {
            Lang::En
        }
    }

    #[cfg(target_os = "linux")]
    pub fn to_group(&self) -> u32 {
        match self {
            Lang::Ru => 1,
            Lang::En => 0,
        }
    }
}
