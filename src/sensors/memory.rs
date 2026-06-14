use cosmic::Element;
use sysinfo::{MemoryRefreshKind, System};

use crate::{
    config::MemoryConfig,
    fl,
};

use cosmic::widget::{settings, toggler};
use std::any::Any;

use cosmic::{
    iced::widget::column,
};

use crate::app::Message;

use bounded_vec_deque::BoundedVecDeque;

use super::Sensor;

const MAX_SAMPLES: usize = 21;

#[derive(Debug)]
pub struct Memory {
    samples_used: BoundedVecDeque<f64>,
    samples_allocated: BoundedVecDeque<f64>,
    total_memory: f64,
    system: System,
    config: MemoryConfig,
}

impl Sensor for Memory {
    fn update_config(&mut self, config: &dyn Any, _refresh_rate: u32) {
        if let Some(cfg) = config.downcast_ref::<MemoryConfig>() {
            self.config = cfg.clone();
        }
    }

    fn update(&mut self) {
        let r = MemoryRefreshKind::nothing().with_ram();

        self.system.refresh_memory_specifics(r);
        let new_val_used: f64 = self.system.used_memory() as f64 / 1_073_741_824.0;
        let new_val_allocated: f64 =
            self.total_memory - (self.system.free_memory() as f64 / 1_073_741_824.0);
        self.samples_used.push_back(new_val_used);
        self.samples_allocated.push_back(new_val_allocated);
    }

    fn settings_ui(&'_ self) -> Element<'_, crate::app::Message> {
        let theme = cosmic::theme::active();
        let cosmic = theme.cosmic();

        let config = &self.config;

        column!(
            settings::item(
                fl!("enable-value"),
                toggler(config.value_visible())
                    .on_toggle(|value| { Message::ToggleMemoryValue(value) }),
            ),
            settings::item(
                fl!("enable-label"),
                toggler(config.label_visible())
                    .on_toggle(|value| { Message::ToggleMemoryLabel(value) }),
            ),
            settings::item(
                fl!("memory-as-percentage"),
                toggler(config.percentage).on_toggle(Message::ToggleMemoryPercentage),
            ),
        )
        .spacing(cosmic.space_xs())
        .into()
    }
}

impl Default for Memory {
    fn default() -> Self {
        let mut system = System::new();
        system.refresh_memory();

        let total_memory: f64 = system.total_memory() as f64 / 1_073_741_824.0;
        log::info!(
            "System memory: {} / {:.2} GB",
            system.total_memory(),
            total_memory
        );

        Memory {
            samples_used: BoundedVecDeque::from_iter(
                std::iter::repeat_n(0.0, MAX_SAMPLES),
                MAX_SAMPLES,
            ),
            samples_allocated: BoundedVecDeque::from_iter(
                std::iter::repeat_n(0.0, MAX_SAMPLES),
                MAX_SAMPLES,
            ),
            total_memory,
            system,
            config: MemoryConfig::default(),
        }
    }
}

impl Memory {
    pub fn latest_sample(&self) -> f64 {
        *self.samples_used.back().unwrap_or(&0f64)
    }

    pub fn latest_sample_allocated(&self) -> f64 {
        *self.samples_allocated.back().unwrap_or(&0f64)
    }

    pub fn total(&self) -> f64 {
        self.total_memory
    }

    pub fn to_string(&self, vertical_panel: bool) -> String {
        let mut current_val = self.latest_sample();
        let unit: &str;

        if self.config.percentage {
            current_val = (current_val * 100.0) / self.total_memory;
            unit = "%";
        } else if !vertical_panel {
            unit = " GB";
        } else {
            unit = "GB";
        }

        if current_val < 10.0 {
            format!("{:.2}{unit}", (current_val * 100.0).trunc() / 100.0)
        } else if current_val < 100.0 {
            format!("{:.1}{unit}", (current_val * 10.0).trunc() / 10.0)
        } else {
            format!("{}{unit}", current_val.round())
        }
    }
}
