use cosmic::Element;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

use crate::{config::GpuConfig, fl};

pub static UNIT_OPTIONS: LazyLock<[&'static str; 4]> = LazyLock::new(|| {
    [
        fl!("temperature-unit-celsius").leak(),
        fl!("temperature-unit-fahrenheit").leak(),
        fl!("temperature-unit-kelvin").leak(),
        fl!("temperature-unit-rankine").leak(),
    ]
});

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TempUnit {
    Celsius,
    Farenheit,
    Kelvin,
    Rankine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuVariant {
    Amd,
    Intel,
}

use std::any::Any;
pub trait Sensor {
    fn update_config(&mut self, config: &dyn Any, refresh_rate: u32);
    fn update(&mut self);
    fn settings_ui(&'_ self) -> Element<'_, crate::app::Message>;
}

pub mod cpu;
pub mod cputemp;
pub mod disks;
pub mod gpu;
pub mod gpus;
pub mod memory;
pub mod network;

impl From<usize> for TempUnit {
    fn from(index: usize) -> Self {
        match index {
            0 => TempUnit::Celsius,
            1 => TempUnit::Farenheit,
            2 => TempUnit::Kelvin,
            3 => TempUnit::Rankine,
            _ => {
                log::error!("Invalid index for TempUnit");
                TempUnit::Celsius
            }
        }
    }
}

impl From<TempUnit> for usize {
    fn from(kind: TempUnit) -> Self {
        match kind {
            TempUnit::Celsius => 0,
            TempUnit::Farenheit => 1,
            TempUnit::Kelvin => 2,
            TempUnit::Rankine => 3,
        }
    }
}
