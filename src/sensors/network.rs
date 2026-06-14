use bounded_vec_deque::BoundedVecDeque;

use cosmic::Element;
use log::info;
use sysinfo::Networks;

use crate::{
    config::{NetworkConfig, NetworkVariant},
    fl,
};

use cosmic::widget;
use cosmic::widget::settings;

use crate::app::Message;
use cosmic::iced::widget::column;
use std::any::Any;

use super::Sensor;

const MAX_SAMPLES: usize = 30;
const UNITS_SHORT: [&str; 5] = ["b", "K", "M", "G", "T"];
const UNITS_LONG: [&str; 5] = ["bps", "Kbps", "Mbps", "Gbps", "Tbps"];
const UNITS_SHORT_BYTES: [&str; 5] = ["B", "K", "M", "G", "T"];
const UNITS_LONG_BYTES: [&str; 5] = ["B/s", "KB/s", "MB/s", "GB/s", "TB/s"];

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum UnitVariant {
    Short,
    Long,
}

#[derive(Debug)]
pub struct Network {
    networks: Networks,
    download: BoundedVecDeque<u64>,
    upload: BoundedVecDeque<u64>,
    config: NetworkConfig,
    refresh_rate: u32,
}

impl Sensor for Network {
    fn update_config(&mut self, config: &dyn Any, refresh_rate: u32) {
        if let Some(cfg) = config.downcast_ref::<NetworkConfig>() {
            self.config = cfg.clone();
            self.refresh_rate = refresh_rate;
        }
    }

    /// Retrieve the amount of data transmitted since last update.
    fn update(&mut self) {
        self.networks.refresh(true);
        let mut dl = 0;
        let mut ul = 0;

        for (_, network) in &self.networks {
            dl += network.received() * 8;
            ul += network.transmitted() * 8;
        }
        self.download.push_back(dl);
        self.upload.push_back(ul);
    }

    fn settings_ui(&'_ self) -> Element<'_, crate::app::Message> {
        let theme = cosmic::theme::active();
        let cosmic = theme.cosmic();

        let k = self.config.variant;
        let config = &self.config;

        let value_label = match k {
            NetworkVariant::Download => fl!("enable-download-value"),
            NetworkVariant::Upload => fl!("enable-upload-value"),
            NetworkVariant::Combined => fl!("enable-value"),
        };

        column!(
            settings::item(
                value_label,
                widget::toggler(config.value_visible())
                    .on_toggle(move |t| Message::ToggleNetValue(k, t)),
            ),
        )
        .spacing(cosmic.space_xs())
        .into()
    }
}

impl Default for Network {
    fn default() -> Self {
        let networks = Networks::new_with_refreshed_list();
        Network {
            networks,
            download: BoundedVecDeque::from_iter(std::iter::repeat_n(0, MAX_SAMPLES), MAX_SAMPLES),
            upload: BoundedVecDeque::from_iter(std::iter::repeat_n(0, MAX_SAMPLES), MAX_SAMPLES),
            config: NetworkConfig::default(),
            refresh_rate: 1000,
        }
    }
}

impl Network {
    fn makestr(val: u64, format: UnitVariant, show_bytes: bool) -> String {
        let mut value = val as f64;

        if show_bytes {
            value /= 8.0;
        }

        let mut unit_index = 0;

        let units = match (show_bytes, format) {
            (false, UnitVariant::Short) => UNITS_SHORT,
            (false, UnitVariant::Long) => UNITS_LONG,
            (true, UnitVariant::Short) => UNITS_SHORT_BYTES,
            (true, UnitVariant::Long) => UNITS_LONG_BYTES,
        };

        // Scale the value to the appropriate unit
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
            info!("Value: {value}. formatted: {value:.2}. string: {value_str}");
            value_str.pop();
        }

        let unit_str = units[unit_index];
        let mut result = String::with_capacity(20);
        result.push_str(&value_str);

        if format == UnitVariant::Long {
            result.push(' ');
        }

        result.push_str(unit_str);

        if format == UnitVariant::Long {
            let padding = 9usize.saturating_sub(result.len());
            if padding > 0 {
                result = " ".repeat(padding) + &result;
            }
        }

        result
    }

    // If the sample rate doesn't match exactly one second (more or less),
    // we grab enough samples to cover it and average the value of samples cover a longer duration.
    fn last_second_bitrate(samples: &BoundedVecDeque<u64>, sample_interval_ms: u32) -> u64 {
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

    // Get bits per second
    pub fn download_label(&self, sample_interval_ms: u32, format: UnitVariant) -> String {
        let rate = Network::last_second_bitrate(&self.download, sample_interval_ms);
        Network::makestr(rate, format, self.config.show_bytes)
    }

    // Get bits per second
    pub fn upload_label(&self, sample_interval_ms: u32, format: UnitVariant) -> String {
        let rate = Network::last_second_bitrate(&self.upload, sample_interval_ms);
        Network::makestr(rate, format, self.config.show_bytes)
    }
}
