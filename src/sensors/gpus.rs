use bounded_vec_deque::BoundedVecDeque;
use cosmic::{Element};
use log::info;
use std::collections::BTreeMap;

use crate::sensors::gpu::GpuType;
use crate::sensors::{GpuConfig, TempUnit};
use cosmic::widget::{self, Column};
use cosmic::widget::{settings, toggler};
use cosmic::{
    iced::{
        Alignment,
        widget::{column, row},
    },
    widget::Row,
};

use crate::app::Message;
use crate::config::DeviceKind;
use crate::{
    config::{GpuTempConfig, GpuUsageConfig, GpuVramConfig},
    fl,
};
use std::any::Any;

use super::gpu::amd::AmdGpu;
use super::gpu::intel::IntelGpu;
use super::gpu::{GpuIf, nvidia::NvidiaGpu};

const MAX_SAMPLES: usize = 21;

pub struct Gpus {
    gpus: BTreeMap<String, Gpu>,
}

impl Gpus {
    pub fn new(is_laptop: bool) -> Self {
        let mut gpus = Self {
            gpus: BTreeMap::new(),
        };

        gpus.redetect(GpuType::Intel, is_laptop);
        gpus.redetect(GpuType::Nvidia, is_laptop);
        gpus.redetect(GpuType::Amd, is_laptop);

        gpus
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Gpu)> {
        self.gpus.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (&String, &mut Gpu)> {
        self.gpus.iter_mut()
    }

    pub fn has_type(&self, gpu_type: GpuType) -> bool {
        self.gpus.values().any(|gpu| gpu.gpu_type() == gpu_type)
    }

    pub fn redetect(&mut self, gpu_type: GpuType, is_laptop: bool) {
        let detected = match gpu_type {
            GpuType::Intel => IntelGpu::get_gpus(),
            GpuType::Nvidia => NvidiaGpu::get_gpus(),
            GpuType::Amd => AmdGpu::get_gpus(),
        };

        for mut gpu in detected {
            let id = gpu.id();

            log::info!(
                "Found GPU. Type: {:?}. Name: {}. UUID: {}",
                gpu.gpu_type(),
                gpu.name(),
                id
            );

            if self.gpus.contains_key(&id) {
                log::info!("Already detected, skipping.");
                continue;
            }

            if is_laptop {
                gpu.set_laptop();
            }

            self.gpus.insert(id, gpu);
        }
    }
    pub fn get(&self, id: &str) -> Option<&Gpu> {
        self.gpus.get(id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Gpu> {
        self.gpus.get_mut(id)
    }

    pub fn values(&self) -> impl Iterator<Item = &Gpu> {
        self.gpus.values()
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut Gpu> {
        self.gpus.values_mut()
    }

    pub fn is_empty(&self) -> bool {
        self.gpus.is_empty()
    }
}

pub struct GpuGraph {
    id: String,
    samples: BoundedVecDeque<f64>,
    disabled: bool,
    config: GpuUsageConfig,
}

impl GpuGraph {
    fn new(id: &str) -> Self {
        GpuGraph {
            id: id.to_owned(),
            samples: BoundedVecDeque::from_iter(std::iter::repeat_n(0.0, MAX_SAMPLES), MAX_SAMPLES),
            disabled: false,
            config: GpuUsageConfig::default(),
        }
    }

    fn update_config(&mut self, config: &dyn Any, _refresh_rate: u32) {
        if let Some(cfg) = config.downcast_ref::<GpuUsageConfig>() {
            self.config = cfg.clone();
        }
    }

    pub fn clear(&mut self) {
        for sample in &mut self.samples {
            *sample = 0.0;
        }
    }

    pub fn latest_sample(&self) -> f64 {
        *self.samples.back().unwrap_or(&0f64)
    }

    pub fn update(&mut self, sample: u32) {
        self.samples.push_back(f64::from(sample));
    }
}

impl fmt::Display for GpuGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.disabled {
            write!(f, "---%")
        } else {
            let current_val = self.latest_sample();
            if current_val < 10.0 {
                write!(f, "{:.2}%", (current_val * 100.0).trunc() / 100.0)
            } else if current_val < 100.0 {
                write!(f, "{:.1}%", (current_val * 10.0).trunc() / 10.0)
            } else {
                write!(f, "{current_val}%")
            }
        }
    }
}

pub struct VramGraph {
    id: String,
    samples: BoundedVecDeque<f64>,
    total: f64,
    disabled: bool,
    config: GpuVramConfig,
}

impl VramGraph {
    // id: a unique id, total: RAM size in GB
    fn new(id: &str, total: f64) -> Self {
        VramGraph {
            id: id.to_owned(),
            samples: BoundedVecDeque::from_iter(std::iter::repeat_n(0.0, MAX_SAMPLES), MAX_SAMPLES),
            total,
            disabled: false,
            config: GpuVramConfig::default(),
        }
    }

    fn update_config(&mut self, config: &dyn Any, _refresh_rate: u32) {
        if let Some(cfg) = config.downcast_ref::<GpuVramConfig>() {
            self.config = cfg.clone();
        }
    }

    pub fn clear(&mut self) {
        for sample in &mut self.samples {
            *sample = 0.0;
        }
    }

    pub fn latest_sample(&self) -> f64 {
        *self.samples.back().unwrap_or(&0f64)
    }

    pub fn string(&self, vertical_panel: bool) -> String {
        let current_val = self.latest_sample();
        let unit: &str = if vertical_panel { "GB" } else { " GB" };

        if self.disabled {
            format!("---{unit}")
        } else if current_val < 10.0 {
            format!("{:.2}{unit}", (current_val * 100.0).trunc() / 100.0)
        } else if current_val < 100.0 {
            format!("{:.1}{unit}", (current_val * 10.0).trunc() / 10.0)
        } else {
            format!("{}{unit}", current_val.round())
        }
    }

    pub fn total(&self) -> f64 {
        self.total
    }

    pub fn update(&mut self, sample: u64) {
        let new_val: f64 = sample as f64 / 1_073_741_824.0;
        self.samples.push_back(new_val);
    }
}

pub struct TempGraph {
    id: String,
    samples: BoundedVecDeque<f64>,
    unit_options: Vec<&'static str>,
    max_temp: f64,
    disabled: bool,
    config: GpuTempConfig,
}

impl TempGraph {
    // id: a unique id, total: RAM size in GB
    fn new(id: &str) -> Self {
        TempGraph {
            id: id.to_owned(),
            samples: BoundedVecDeque::from_iter(std::iter::repeat_n(0.0, MAX_SAMPLES), MAX_SAMPLES),
            unit_options: super::UNIT_OPTIONS.to_vec(),
            max_temp: 100.0,
            disabled: false,
            config: GpuTempConfig::default(),
        }
    }

    fn update_config(&mut self, config: &dyn Any, _refresh_rate: u32) {
        if let Some(cfg) = config.downcast_ref::<GpuTempConfig>() {
            self.config = cfg.clone();
        }
    }

    pub fn clear(&mut self) {
        for sample in &mut self.samples {
            *sample = 0.0;
        }
    }

    pub fn latest_sample(&self) -> f64 {
        *self.samples.back().unwrap_or(&0f64)
    }

    pub fn update(&mut self, sample: u32) {
        let new_val = f64::from(sample) / 1000.0;
        self.samples.push_back(new_val);
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

impl fmt::Display for TempGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let current_val = self.latest_sample();
        if self.disabled || current_val <= 0.0 {
            match self.config.unit {
                TempUnit::Celsius => write!(f, "--°C"),
                TempUnit::Farenheit => write!(f, "---°F"),
                TempUnit::Kelvin => write!(f, "---K"),
                TempUnit::Rankine => write!(f, "---°R"),
            }
        } else {
            match self.config.unit {
                TempUnit::Celsius => write!(f, "{}°C", current_val.trunc()),
                TempUnit::Farenheit => write!(f, "{}°F", (current_val * 9.0 / 5.0 + 32.0).trunc()),
                TempUnit::Kelvin => write!(f, "{}K", (current_val + 273.15).trunc()),
                TempUnit::Rankine => write!(f, "{}°R", (current_val * 9.0 / 5.0 + 491.67).trunc()),
            }
        }
    }
}

pub struct Gpu {
    gpu_if: Box<dyn GpuIf>,
    pub gpu: GpuGraph,
    pub vram: VramGraph,
    pub temp: TempGraph,
    is_laptop: bool,
    config: GpuConfig,
}

impl Gpu {
    pub fn new(gpu_if: Box<dyn GpuIf>) -> Self {
        let total = gpu_if.vram_total();
        let id = gpu_if.id();

        Gpu {
            gpu_if,
            gpu: GpuGraph::new(&id),
            vram: VramGraph::new(&id, total as f64 / 1_073_741_824.0),
            temp: TempGraph::new(&id),
            is_laptop: false,
            config: GpuConfig::default(),
        }
    }

    pub fn update_config(&mut self, config: &dyn Any, refresh_rate: u32) {
        if let Some(cfg) = config.downcast_ref::<GpuConfig>() {
            self.config = cfg.clone();
            self.gpu.update_config(&cfg.usage, refresh_rate);
            self.vram.update_config(&cfg.vram, refresh_rate);
            self.temp.update_config(&cfg.temp, refresh_rate);
        }
    }

    pub fn name(&self) -> String {
        self.gpu_if.as_ref().name().clone()
    }

    pub fn id(&self) -> String {
        self.gpu_if.as_ref().id().clone()
    }

    pub fn set_laptop(&mut self) {
        self.is_laptop = true;
    }

    pub fn update(&mut self) {
        if self.gpu_if.is_active() {
            if let Ok(sample) = self.gpu_if.usage() {
                self.gpu.update(sample);
            }
            if let Ok(sample) = self.gpu_if.vram_used() {
                self.vram.update(sample);
            }
            if let Ok(sample) = self.gpu_if.temperature() {
                self.temp.update(sample);
            }
        }
    }

    pub fn restart(&mut self) {
        info!("Restarting {}", self.name());
        self.gpu_if.restart();
        self.gpu.disabled = false;
        self.vram.disabled = false;
        self.temp.disabled = false;
    }

    pub fn stop(&mut self) {
        info!("Stopping {}", self.name());
        self.gpu_if.stop();
        self.gpu.clear();
        self.vram.clear();
        self.temp.clear();
        self.gpu.disabled = true;
        self.vram.disabled = true;
        self.temp.disabled = true;
    }

    pub fn is_active(&self) -> bool {
        self.gpu_if.is_active()
    }

    pub fn gpu_type(&self) -> GpuType {
        self.gpu_if.gpu_type()
    }

    fn settings_usage_ui(
        &'_ self,
        config: &crate::config::GpuUsageConfig,
    ) -> Element<'_, crate::app::Message> {
        let theme = cosmic::theme::active();
        let cosmic = theme.cosmic();

        let mut gpu_elements = Vec::new();

        let usage = self.gpu.to_string();
        gpu_elements.push(Element::from(
            column!(
                cosmic::widget::text::body(usage.to_string())
                    .width(90)
                    .align_x(Alignment::Center)
            )
            .padding(cosmic::theme::spacing().space_xs)
            .align_x(Alignment::Center),
        ));

        let _id = self.id();
        gpu_elements.push(Element::from(
            column!(
                settings::item(
                    fl!("enable-value"),
                    toggler(config.value_visible()).on_toggle(move |value| {
                        Message::GpuToggleValue(self.id(), DeviceKind::Gpu, value)
                    }),
                ),
            )
            .spacing(cosmic.space_xs()),
        ));

        column![
            widget::text::heading(fl!("gpu-title-usage")),
            Row::with_children(gpu_elements)
                .align_y(Alignment::Center)
                .spacing(cosmic.space_xs())
        ]
        .spacing(cosmic::theme::spacing().space_xs)
        .into()
    }

    fn settings_vram_ui(
        &'_ self,
        config: &crate::config::GpuVramConfig,
    ) -> Element<'_, crate::app::Message> {
        let theme = cosmic::theme::active();
        let cosmic = theme.cosmic();

        // VRAM load
        let mut vram_elements = Vec::new();
        let vram = self.vram.string(false);
        vram_elements.push(Element::from(
            column!(
                cosmic::widget::text::body(vram.to_string())
                    .width(90)
                    .align_x(Alignment::Center)
            )
            .padding(cosmic::theme::spacing().space_xs)
            .align_x(Alignment::Center),
        ));

        vram_elements.push(Element::from(
            column!(
                settings::item(
                    fl!("enable-value"),
                    toggler(config.value_visible()).on_toggle(|value| {
                        Message::GpuToggleValue(self.id(), DeviceKind::Vram, value)
                    }),
                ),
            )
            .spacing(cosmic.space_xs()),
        ));

        column![
            widget::text::heading(fl!("gpu-title-vram")),
            Row::with_children(vram_elements)
                .align_y(Alignment::Center)
                .spacing(cosmic.space_xs())
        ]
        .spacing(cosmic::theme::spacing().space_xs)
        .into()
    }

    fn settings_temp_ui(
        &'_ self,
        config: &crate::config::GpuTempConfig,
    ) -> Element<'_, crate::app::Message> {
        let theme = cosmic::theme::active();
        let cosmic = theme.cosmic();

        // GPU temperature
        let mut temp_elements = Vec::new();
        let temp = self.temp.to_string();
        temp_elements.push(Element::from(
            column!(
                cosmic::widget::text::body(temp.to_string())
                    .width(90)
                    .align_x(Alignment::Center)
            )
            .padding(cosmic::theme::spacing().space_xs)
            .align_x(Alignment::Center),
        ));

        let selected_unit: Option<usize> = Some(self.temp.config.unit.into());
        let id1 = self.id();

        temp_elements.push(Element::from(
            column!(
                settings::item(
                    fl!("enable-value"),
                    toggler(config.value_visible()).on_toggle(|value| {
                        Message::GpuToggleValue(self.id(), DeviceKind::GpuTemp, value)
                    }),
                ),
                settings::item(
                    fl!("temperature-unit"),
                    widget::dropdown(&self.temp.unit_options, selected_unit, move |m| {
                        Message::SelectGpuTempUnit(id1.clone(), m.into())
                    },)
                ),
            )
            .spacing(cosmic.space_xs()),
        ));

        column![
            widget::text::heading(fl!("gpu-title-temperature")),
            Row::with_children(temp_elements)
                .align_y(Alignment::Center)
                .spacing(cosmic.space_xs())
        ]
        .spacing(cosmic::theme::spacing().space_xs)
        .into()
    }

    pub fn settings_ui(
        &'_ self,
        config: &crate::config::GpuConfig,
    ) -> cosmic::Element<'_, crate::app::Message> {
        let battery_disable = if self.is_laptop {
            Some(
                settings::item(
                    fl!("settings-disable-on-battery"),
                    widget::checkbox(config.pause_on_battery).on_toggle(move |value| {
                        Message::ToggleDisableOnBattery(self.id().clone(), value)
                    }),
                )
                .width(340),
            )
        } else {
            None
        };

        let label_toggle = settings::item(
            fl!("enable-label"),
            widget::toggler(config.usage.label_visible())
                .on_toggle(move |value| Message::GpuToggleLabel(self.id().clone(), value)),
        );

        let usage = self.settings_usage_ui(&config.usage);
        let vram = self.settings_vram_ui(&config.vram);

        let stacked = if config.vram.value_visible() && config.usage.value_visible() {
            Some(settings::item(
                fl!("settings-gpu-stack-values"),
                row!(
                    widget::toggler(config.stack_values).on_toggle(move |value| {
                        Message::GpuToggleStackValues(self.id().clone(), value)
                    })
                ),
            ))
        } else {
            None
        };

        let temp = self.settings_temp_ui(&config.temp);

        Column::new()
            .push_maybe(battery_disable)
            .push(label_toggle)
            .push(usage)
            .push(temp)
            .push(vram)
            .push_maybe(stacked)
            .spacing(cosmic::theme::spacing().space_xs)
            .into()
    }
}
