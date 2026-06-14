use crate::{
    config::CpuConfig,
    fl,
};
use bounded_vec_deque::BoundedVecDeque;
use cosmic::{
    Element, widget::Column,
};
use std::any::Any;

use cosmic::widget::{self, settings, toggler};

use cosmic::iced::widget::row;

use crate::app::Message;

use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use super::Sensor;

const MAX_SAMPLES: usize = 21;

#[derive(Debug, Clone, Copy, Default)]
struct CpuStat {
    user: u64,
    nice: u64,
    system: u64,
    idle: u64,
    iowait: u64,
    irq: u64,
    softirq: u64,
    steal: u64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CpuLoad {
    pub user_pct: f64,
    pub system_pct: f64,
}

#[derive(Debug)]
pub struct Cpu {
    // Total CPU load since last update split into user and system
    total_cpu_load: CpuLoad,
    // Load per core since last update split into user and system
    core_loads: HashMap<usize, CpuLoad>,
    // Current Load per core since /proc
    current_core_stats: HashMap<usize, CpuStat>,
    // Load per core in last update
    prev_core_stats: HashMap<usize, CpuStat>,
    // Total CPU load for the last MAX_SAMPLES updates
    samples_sum: BoundedVecDeque<f64>,
    // CPU load for the last MAX_SAMPLES updates, split into user and system
    samples_split: BoundedVecDeque<CpuLoad>,
    config: CpuConfig,
}

impl Sensor for Cpu {
    fn update_config(&mut self, config: &dyn Any, _refresh_rate: u32) {
        if let Some(cfg) = config.downcast_ref::<CpuConfig>() {
            self.config = cfg.clone();
        }
    }

    fn update(&mut self) {
        self.update_stats();
        self.samples_split.push_back(self.total_cpu_load);
        self.samples_sum
            .push_back(self.total_cpu_load.user_pct + self.total_cpu_load.system_pct);
    }

    fn settings_ui(&'_ self) -> Element<'_, crate::app::Message> {
        let theme = cosmic::theme::active();
        let cosmic = theme.cosmic();

        let mut cpu_column = Vec::new();

        let config = &self.config;

        cpu_column.push(
            settings::item(
                fl!("enable-value"),
                toggler(config.value_visible()).on_toggle(Message::ToggleCpuValue),
            )
            .into(),
        );
        cpu_column.push(
            settings::item(
                fl!("enable-label"),
                toggler(config.label_visible()).on_toggle(Message::ToggleCpuLabel),
            )
            .into(),
        );
        if self.config.value_visible() {
            cpu_column.push(
                settings::item(
                    fl!("cpu-no-decimals"),
                    row!(
                        widget::checkbox(config.no_decimals)
                            .on_toggle(Message::ToggleCpuNoDecimals)
                    ),
                )
                .into(),
            );
        }

        Column::with_children(cpu_column)
            .spacing(cosmic.space_xs())
            .into()
    }
}

impl Cpu {
    pub fn new(_is_horizontal: bool) -> Self {
        // Initialize CPU/Core structures
        let mut core_stats: HashMap<usize, CpuStat> = HashMap::new();
        Self::read_cpu_stats(&mut core_stats);
        log::info!("Found CPU Cores: {}", core_stats.len());

        let core_loads: HashMap<usize, CpuLoad> = core_stats
            .keys()
            .map(|&k| (k, CpuLoad::default()))
            .collect();

        let cpu = Cpu {
            total_cpu_load: CpuLoad {
                user_pct: 0.,
                system_pct: 0.,
            },
            core_loads,
            current_core_stats: core_stats.clone(),
            prev_core_stats: core_stats,
            samples_sum: BoundedVecDeque::from_iter(
                std::iter::repeat_n(0.0, MAX_SAMPLES),
                MAX_SAMPLES,
            ),
            samples_split: BoundedVecDeque::from_iter(
                std::iter::repeat_n(
                    CpuLoad {
                        user_pct: 0.,
                        system_pct: 0.,
                    },
                    MAX_SAMPLES,
                ),
                MAX_SAMPLES,
            ),
            config: CpuConfig::default(),
        };
        cpu
    }

    pub fn latest_sample(&self) -> f64 {
        *self.samples_sum.back().unwrap_or(&0f64)
    }

    pub fn core_count(&self) -> usize {
        self.core_loads.len()
    }

    fn read_cpu_stats(cpu_stats: &mut HashMap<usize, CpuStat>) {
        let Ok(file) = File::open(Path::new("/proc/stat")) else {
            return;
        };

        let reader = BufReader::new(file);
        cpu_stats.clear();

        for line in reader.lines() {
            let Ok(line) = line else { continue };
            let parts: Vec<&str> = line.split_whitespace().collect();

            if parts.is_empty() || !parts[0].starts_with("cpu") || parts[0] == "cpu" {
                continue;
            }

            let Ok(core_num) = parts[0].trim_start_matches("cpu").parse::<usize>() else {
                continue;
            };

            if parts.len() < 9 {
                continue;
            }

            let user = parts[1].parse::<u64>().unwrap_or(0);
            let nice = parts[2].parse::<u64>().unwrap_or(0);
            let system = parts[3].parse::<u64>().unwrap_or(0);
            let idle = parts[4].parse::<u64>().unwrap_or(0);
            let iowait = parts[5].parse::<u64>().unwrap_or(0);
            let irq = parts[6].parse::<u64>().unwrap_or(0);
            let softirq = parts[7].parse::<u64>().unwrap_or(0);
            let steal = parts[8].parse::<u64>().unwrap_or(0);

            let core_stats = CpuStat {
                user,
                nice,
                system,
                idle,
                iowait,
                irq,
                softirq,
                steal,
            };

            cpu_stats.insert(core_num, core_stats);
        }
    }

    fn update_stats(&mut self) {
        self.current_core_stats.clear();
        Cpu::read_cpu_stats(&mut self.current_core_stats);

        let mut total_user_pct = 0.0;
        let mut total_system_pct = 0.0;
        let mut counted_cores = 0;

        self.core_loads.clear();

        for (&core_num, current) in &self.current_core_stats {
            if let Some(prev) = self.prev_core_stats.get_mut(&core_num) {
                let user = current.user.saturating_sub(prev.user);
                let nice = current.nice.saturating_sub(prev.nice);
                let system = current.system.saturating_sub(prev.system);
                let idle = current.idle.saturating_sub(prev.idle);
                let iowait = current.iowait.saturating_sub(prev.iowait);
                let irq = current.irq.saturating_sub(prev.irq);
                let softirq = current.softirq.saturating_sub(prev.softirq);
                let steal = current.steal.saturating_sub(prev.steal);

                let total = user + nice + system + idle + iowait + irq + softirq + steal;
                if total == 0 {
                    continue;
                }

                let total_f64 = total as f64;
                let user_pct = (user + nice) as f64 / total_f64 * 100.0;
                let system_pct = system as f64 / total_f64 * 100.0;

                self.core_loads.insert(
                    core_num,
                    CpuLoad {
                        user_pct,
                        system_pct,
                    },
                );

                total_user_pct += user_pct;
                total_system_pct += system_pct;
                counted_cores += 1;

                *prev = *current;
            }
        }

        if counted_cores > 0 {
            let core_count_f64 = f64::from(counted_cores);
            self.total_cpu_load = CpuLoad {
                user_pct: total_user_pct / core_count_f64,
                system_pct: total_system_pct / core_count_f64,
            };
        }
    }
}

use std::fmt;

impl fmt::Display for Cpu {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let current_val = self.latest_sample();

        if self.config.no_decimals {
            write!(f, "{}%", current_val.round())
        } else if current_val < 10.0 {
            write!(f, "{current_val:.2}%")
        } else if current_val < 100.0 {
            write!(f, "{current_val:.1}%")
        } else {
            write!(f, "{current_val}%")
        }
    }
}
