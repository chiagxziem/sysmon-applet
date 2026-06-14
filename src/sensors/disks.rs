use bounded_vec_deque::BoundedVecDeque;

use sysinfo::{DiskRefreshKind, Disks as DisksInfo};

use crate::{
    config::{DisksConfig, DisksVariant},
    fl,
};

use cosmic::Element;

use cosmic::widget;
use cosmic::widget::settings;

use cosmic::iced::widget::column;

use crate::app::Message;
use std::any::Any;

use super::Sensor;

const MAX_SAMPLES: usize = 30;
const UNITS_SHORT: [&str; 5] = ["B", "K", "M", "G", "T"];
const UNITS_LONG: [&str; 5] = ["B/s", "KB/s", "MB/s", "GB/s", "TB/s"];

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum UnitVariant {
    Short,
    Long,
}

#[derive(Debug)]
pub struct Disks {
    disks: DisksInfo,
    write: BoundedVecDeque<u64>,
    read: BoundedVecDeque<u64>,
    config: DisksConfig,
    refresh_rate: u32,
}

impl Sensor for Disks {
    fn update_config(&mut self, config: &dyn Any, refresh_rate: u32) {
        if let Some(cfg) = config.downcast_ref::<DisksConfig>() {
            self.config = cfg.clone();
            self.refresh_rate = refresh_rate;
        }
    }

    /// Retrieve the amount of data transmitted since last update.
    fn update(&mut self) {
        let r = DiskRefreshKind::nothing().with_io_usage();
        self.disks.refresh_specifics(true, r);
        let mut wr = 0;
        let mut rd = 0;

        for disk in self.disks.list() {
            let usage = disk.usage();
            wr += usage.written_bytes;
            rd += usage.read_bytes;
        }

        self.write.push_back(wr);
        self.read.push_back(rd);
    }

    fn settings_ui(&'_ self) -> Element<'_, crate::app::Message> {
        let theme = cosmic::theme::active();
        let cosmic = theme.cosmic();

        let k = self.config.variant;
        let config = &self.config;

        let value_label = match k {
            DisksVariant::Write => fl!("enable-write-value"),
            DisksVariant::Read => fl!("enable-read-value"),
            DisksVariant::Combined => fl!("enable-value"),
        };

        column!(
            settings::item(
                value_label,
                widget::toggler(config.value_visible())
                    .on_toggle(move |t| Message::ToggleDisksValue(k, t)),
            ),
        )
        .spacing(cosmic.space_xs())
        .into()
    }
}

impl Default for Disks {
    fn default() -> Self {
        let disks = DisksInfo::new_with_refreshed_list();
        Disks {
            disks,
            write: BoundedVecDeque::from_iter(std::iter::repeat_n(0, MAX_SAMPLES), MAX_SAMPLES),
            read: BoundedVecDeque::from_iter(std::iter::repeat_n(0, MAX_SAMPLES), MAX_SAMPLES),
            config: DisksConfig::default(),
            refresh_rate: 1000,
        }
    }
}

impl Disks {
    fn makestr(val: u64, format: UnitVariant) -> String {
        let mut formatted = String::with_capacity(20);

        let mut value = val as f64;
        let mut unit_index = 0;
        let units = if format == UnitVariant::Short {
            UNITS_SHORT
        } else {
            UNITS_LONG
        };

        // Find the appropriate unit
        while value >= 999.0 && unit_index < units.len() - 1 {
            value /= 1000.0;
            unit_index += 1;
        }

        // Format the number with varying precision, prevent the formatter from rounding up
        let mut value_str = if value < 10.0 {
            format!("{:.2}", (value * 100.0).trunc() / 100.0)
        } else if value < 100.0 {
            format!("{:.1}", (value * 10.0).trunc() / 10.0)
        } else {
            format!("{:.0}", value.trunc())
        };

        // This happens when value is something like 9.9543456789908765453456 and it's rounded up to 10.
        if value_str.len() == 5 {
            log::info!("Value: {value}. formatted: {value:.2}. string: {value_str}");
            value_str.pop();
        }

        formatted.push_str(&value_str);

        if format == UnitVariant::Long {
            formatted.push(' ');
        }

        formatted.push_str(units[unit_index]);

        if format == UnitVariant::Long {
            let padding = 9usize.saturating_sub(formatted.len());
            if padding > 0 {
                formatted = " ".repeat(padding) + &formatted;
            }
        }

        formatted
    }

    // If the sample rate doesn't match exactly one second (more or less),
    // we grab enough samples to cover it and average the value of samples cover a longer duration.
    fn last_second_rate(samples: &BoundedVecDeque<u64>, sample_interval_ms: u32) -> u64 {
        let mut total_duration = 0u32;
        let mut total_bitrate = 0u64;

        // Iterate from newest to oldest
        for &bitrate in samples.iter().rev() {
            if total_duration >= 1000 {
                break;
            }

            total_bitrate += bitrate;
            total_duration += sample_interval_ms;
        }

        // Scale to exactly 1000ms
        let scale = 1000.0 / f64::from(total_duration);

        (total_bitrate as f64 * scale).floor() as u64
    }

    // Get bytes per second
    pub fn write_label(&self, sample_interval_ms: u32, format: UnitVariant) -> String {
        let val = Disks::last_second_rate(&self.write, sample_interval_ms);
        Disks::makestr(val, format)
    }

    // Get bytes per second
    pub fn read_label(&self, sample_interval_ms: u32, format: UnitVariant) -> String {
        let val = Disks::last_second_rate(&self.read, sample_interval_ms);
        Disks::makestr(val, format)
    }
}
