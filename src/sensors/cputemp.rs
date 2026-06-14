use crate::{
    config::CpuTempConfig,
    fl,
};
use cosmic::{Element};

use cosmic::widget;
use cosmic::widget::{settings, toggler};

use cosmic::{
    iced::{
        Alignment,
        widget::{column},
    },
    widget::Row,
};
use log::info;

use crate::app::Message;
use std::any::Any;

use bounded_vec_deque::BoundedVecDeque;
use std::{
    fs,
    path::{Path, PathBuf},
};

use std::fs::read_dir;
use std::io;

use super::{CpuVariant, Sensor, TempUnit};

const MAX_SAMPLES: usize = 21;

#[derive(Debug)]
pub struct HwmonTemp {
    pub temp_paths: Vec<PathBuf>,
    pub crit_temp: f64,
    pub cpu: super::CpuVariant,
}

impl HwmonTemp {
    /// Initialize and return the most relevant CPU temperature sensors
    pub fn find_cpu_sensor() -> io::Result<Option<HwmonTemp>> {
        info!("Find CPU temperature sensor");
        let hwmon_base = Path::new("/sys/class/hwmon");

        for entry in read_dir(hwmon_base)? {
            let hwmon = entry?.path();
            let name_path = hwmon.join("name");

            let Ok(name) = fs::read_to_string(&name_path) else {
                continue;
            };
            let name = name.trim().to_lowercase();
            info!("  path: {name_path:?}. name: {name}");

            if name.contains("coretemp")
                || name.contains("k10temp")
                || name.contains("cpu")
                || name.contains("zenpower")
            {
                let mut tdie: Option<(PathBuf, String)> = None;
                let mut tctl: Option<(PathBuf, String)> = None;
                let mut ccd: Option<(PathBuf, String)> = None;
                let mut core_fallbacks = vec![];

                for i in 0..100 {
                    let label_path = hwmon.join(format!("temp{i}_label"));
                    let input_path = hwmon.join(format!("temp{i}_input"));

                    if !input_path.exists() {
                        continue;
                    }
                    if let Ok(label) = fs::read_to_string(&label_path) {
                        let label = label.trim();

                        if label.eq_ignore_ascii_case("Tdie") {
                            info!("  found sensor {label_path:?} {label}");
                            tdie = Some((input_path.clone(), label.to_string()));
                        } else if label.eq_ignore_ascii_case("Tctl") {
                            info!("  found sensor {label_path:?} {label}");
                            tctl = Some((input_path.clone(), label.to_string()));
                        } else if label.eq_ignore_ascii_case("ccd") {
                            info!("  found sensor {label_path:?} {label}");
                            ccd = Some((input_path.clone(), label.to_string()));
                        } else if label.starts_with("Core") || label.contains("Package") {
                            info!("  found sensor {label_path:?} {label}");
                            core_fallbacks.push((input_path.clone(), label.to_string()));
                        }
                    }
                }

                // Prioritize Tdie > Tctl
                if let Some((path, _label)) = tdie.or(ccd).or(tctl) {
                    let crit_path = hwmon.join("temp1_crit");
                    let crit_temp = fs::read_to_string(&crit_path)
                        .ok()
                        .and_then(|v| v.trim().parse::<f64>().ok())
                        .map_or(100.0, |v| v / 1000.0);

                    return Ok(Some(HwmonTemp {
                        temp_paths: vec![path.clone()],
                        crit_temp,
                        cpu: CpuVariant::Amd,
                    }));
                } else if !core_fallbacks.is_empty() {
                    return Ok(Some(HwmonTemp {
                        temp_paths: core_fallbacks.iter().map(|(p, _)| p.clone()).collect(),
                        crit_temp: 100.0,
                        cpu: CpuVariant::Intel,
                    }));
                }
            }
        }

        Ok(None)
    }

    /// Read current max temperature from all tracked sensor paths
    pub fn read_temp(&self) -> io::Result<f32> {
        let mut max_temp = f32::MIN;

        for path in &self.temp_paths {
            let raw = fs::read_to_string(path)?;
            let millideg: i32 = raw.trim().parse().map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidData, format!("Parse error: {e}"))
            })?;
            let temp_c = millideg as f32 / 1000.0;
            max_temp = max_temp.max(temp_c);
        }

        Ok(max_temp)
    }
}

#[derive(Debug)]
pub struct CpuTemp {
    hwmon_temp: Option<HwmonTemp>,
    pub samples: BoundedVecDeque<f64>,
    unit_options: Vec<&'static str>,
    config: CpuTempConfig,
}

impl Sensor for CpuTemp {
    fn update_config(&mut self, config: &dyn Any, _refresh_rate: u32) {
        if let Some(cfg) = config.downcast_ref::<CpuTempConfig>() {
            self.config = cfg.clone();
        }
    }

    fn update(&mut self) {
        if let Some(hw) = &self.hwmon_temp {
            match hw.read_temp() {
                Ok(temp) => {
                    self.samples.push_back(f64::from(temp));
                }
                Err(e) => info!("Error reading temp data {e:?}"),
            }
        }
    }

    fn settings_ui(&'_ self) -> Element<'_, crate::app::Message> {
        let theme = cosmic::theme::active();
        let cosmic = theme.cosmic();

        let mut temp_elements = Vec::new();

        let selected_unit: Option<usize> = Some(self.config.unit.into());

        let config = &self.config;

        temp_elements.push(Element::from(
            column!(
                settings::item(
                    fl!("enable-value"),
                    toggler(config.value_visible())
                        .on_toggle(|value| { Message::ToggleCpuTempValue(value) }),
                ),
                settings::item(
                    fl!("enable-label"),
                    toggler(config.label_visible())
                        .on_toggle(|value| { Message::ToggleCpuTempLabel(value) }),
                ),
                settings::item(
                    fl!("temperature-unit"),
                    widget::dropdown(&self.unit_options, selected_unit, |m| {
                        Message::SelectCpuTempUnit(m.into())
                    },)
                ),
            )
            .spacing(cosmic.space_xs()),
        ));

        let mut expl = String::with_capacity(128);
        if let Some(hw) = &self.hwmon_temp {
            if hw.cpu == super::CpuVariant::Amd {
                expl.push_str(&fl!("cpu-temp-amd"));
            } else {
                expl.push_str(&fl!("cpu-temp-intel"));
            }
        }

        column!(
            Element::from(widget::text::body(expl)),
            Element::from(
                Row::with_children(temp_elements)
                    .align_y(Alignment::Center)
                    .spacing(0)
            )
        )
        .spacing(10)
        .into()
    }
}

impl Default for CpuTemp {
    fn default() -> Self {
        let mut hwmon = None;

        match HwmonTemp::find_cpu_sensor() {
            Ok(hwmon_option) => {
                hwmon = hwmon_option;
                if hwmon.is_none() {
                    info!("CpuTemp:detect: No CPU Temp IF found.");
                }
            }
            Err(e) => info!("CpuTemp:detect: No CPU Temp IF found. {e:?}"),
        }

        CpuTemp {
            hwmon_temp: hwmon,
            samples: BoundedVecDeque::from_iter(std::iter::repeat_n(0.0, MAX_SAMPLES), MAX_SAMPLES),
            unit_options: super::UNIT_OPTIONS.to_vec(),
            config: CpuTempConfig::default(),
        }
    }
}

impl CpuTemp {
    // true if a CPU temperature hwmon path was found
    pub fn is_found(&self) -> bool {
        self.hwmon_temp.is_some()
    }

    pub fn latest_sample(&self) -> f64 {
        *self.samples.back().unwrap_or(&0f64)
    }

    pub fn to_string_raw(&self) -> String {
        let current_val = self.latest_sample();
        match self.config.unit {
            TempUnit::Celsius => current_val.trunc().to_string(),
            TempUnit::Farenheit => (current_val * 9.0 / 5.0 + 32.0).trunc().to_string(),
            TempUnit::Kelvin => (current_val + 273.15).trunc().to_string(),
            TempUnit::Rankine => (current_val * 9.0 / 5.0 + 491.67).trunc().to_string(),
        }
    }
}

use std::fmt;

impl fmt::Display for CpuTemp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let current_val = self.latest_sample();
        match self.config.unit {
            TempUnit::Celsius => write!(f, "{}°C", current_val.trunc()),
            TempUnit::Farenheit => write!(f, "{}°F", (current_val * 9.0 / 5.0 + 32.0).trunc()),
            TempUnit::Kelvin => write!(f, "{}K", (current_val + 273.15).trunc()),
            TempUnit::Rankine => write!(f, "{}°R", (current_val * 9.0 / 5.0 + 491.67).trunc()),
        }
    }
}
