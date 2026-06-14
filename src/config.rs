use std::collections::HashMap;

use cosmic::cosmic_config::{self, CosmicConfigEntry, cosmic_config_derive::CosmicConfigEntry};
use serde::{Deserialize, Serialize};

use crate::{fl, sensors::TempUnit};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    Cpu,
    CpuTemp,
    Memory,
    Network(NetworkVariant),
    Disks(DisksVariant),
    Gpu,
    Vram,
    GpuTemp,
}

impl std::fmt::Display for DeviceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DeviceKind::Cpu => write!(f, "{}", fl!("sensor-cpu")),
            DeviceKind::CpuTemp => write!(f, "{}", fl!("sensor-cpu-temperature")),
            DeviceKind::Memory => write!(f, "{}", fl!("sensor-memory")),
            DeviceKind::Network(_) => write!(f, "{}", fl!("sensor-network")),
            DeviceKind::Disks(_) => write!(f, "{}", fl!("sensor-disks")),
            DeviceKind::Gpu => write!(f, "{}", fl!("sensor-gpu")),
            DeviceKind::Vram => write!(f, "{}", fl!("sensor-vram")),
            DeviceKind::GpuTemp => write!(f, "{}", fl!("sensor-gpu-temp")),
        }
    }
}

macro_rules! make_config {
    ($name:ident { $($extra:tt)* }) => {
        #[derive(Debug, Clone, Serialize, Deserialize, CosmicConfigEntry, PartialEq)]
        #[serde(default)]
        #[version = 1]
        pub struct $name {
            value_visible: bool,
            label_visible: bool,
            $($extra)*
        }

       impl $name {
            pub fn visible(&self) -> bool {
                self.value_visible
            }
            pub fn value_visible(&self) -> bool {
                self.value_visible
            }
            pub fn label_visible(&self) -> bool {
                self.label_visible
            }
            pub fn show_value(&mut self, visible: bool) {
                self.value_visible = visible;
            }
            pub fn show_label(&mut self, visible: bool) {
                self.label_visible = visible;
            }
        }
    };
}

make_config!(CpuConfig {
    pub no_decimals: bool,
});

impl Default for CpuConfig {
    fn default() -> Self {
        Self {
            value_visible: false,
            label_visible: false,

            no_decimals: false,
        }
    }
}

make_config!(CpuTempConfig {
    pub unit: TempUnit,
});

impl Default for CpuTempConfig {
    fn default() -> Self {
        Self {
            value_visible: false,
            label_visible: false,

            unit: TempUnit::Celsius,
        }
    }
}

make_config!(MemoryConfig {
    pub percentage: bool,
    pub show_allocated: bool,
    pub stack_values: bool,
});

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            value_visible: false,
            label_visible: false,

            percentage: false,
            show_allocated: false,
            stack_values: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum NetworkVariant {
    Download,
    Upload,
    Combined,
}

make_config!(NetworkConfig {
    pub variant: NetworkVariant,
    pub show_bytes: bool,
});

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            value_visible: false,
            label_visible: false,

            variant: NetworkVariant::Combined,
            show_bytes: false,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DisksVariant {
    Write,
    Read,
    Combined,
}

make_config!(DisksConfig {
    pub variant: DisksVariant,
});

impl Default for DisksConfig {
    fn default() -> Self {
        Self {
            value_visible: false,
            label_visible: false,

            variant: DisksVariant::Combined,
        }
    }
}

make_config!(GpuUsageConfig {});

impl Default for GpuUsageConfig {
    fn default() -> Self {
        Self {
            value_visible: false,
            label_visible: false,

        }
    }
}

make_config!(GpuVramConfig {});

impl Default for GpuVramConfig {
    fn default() -> Self {
        Self {
            value_visible: false,
            label_visible: false,

        }
    }
}

make_config!(GpuTempConfig {
    pub unit: TempUnit,
});

impl Default for GpuTempConfig {
    fn default() -> Self {
        Self {
            value_visible: false,
            label_visible: false,

            unit: TempUnit::Celsius,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, CosmicConfigEntry, PartialEq)]
#[version = 1]
pub struct GpuConfig {
    pub usage: GpuUsageConfig,
    pub vram: GpuVramConfig,
    pub temp: GpuTempConfig,
    pub pause_on_battery: bool,
    pub stack_values: bool,
}

impl GpuConfig {
    pub fn is_visible(&self) -> bool {
        self.usage.visible() || self.vram.visible() || self.temp.visible()
    }
}

impl Default for GpuConfig {
    fn default() -> Self {
        Self {
            usage: GpuUsageConfig::default(),
            vram: GpuVramConfig::default(),
            temp: GpuTempConfig::default(),
            pause_on_battery: true,
            stack_values: true,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum ContentType {
    CpuUsage,
    CpuTemp,
    MemoryUsage,
    NetworkUsage,
    DiskUsage,
    GpuInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, CosmicConfigEntry, PartialEq, Eq)]
#[version = 1]
pub struct ContentOrder {
    pub order: Vec<ContentType>,
}

impl Default for ContentOrder {
    fn default() -> Self {
        Self {
            order: vec![
                ContentType::CpuUsage,
                ContentType::CpuTemp,
                ContentType::MemoryUsage,
                ContentType::NetworkUsage,
                ContentType::DiskUsage,
                ContentType::GpuInfo,
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, CosmicConfigEntry, PartialEq)]
#[version = 1]
pub struct SysmonConfig {
    pub refresh_rate: u32,
    pub value_size_default: u16,
    pub label_size_default: u16,
    pub combined_value_size_default: u16,
    pub monospace_values: bool,

    pub cpu: CpuConfig,
    pub cputemp: CpuTempConfig,
    pub memory: MemoryConfig,

    pub network1: NetworkConfig,
    pub network2: NetworkConfig,

    pub disks1: DisksConfig,
    pub disks2: DisksConfig,

    pub gpus: HashMap<String, GpuConfig>,

    pub sysmon: Option<String>,

    pub panel_spacing: u16,

    pub content_order: ContentOrder,
}

impl Default for SysmonConfig {
    fn default() -> Self {
        Self {
            refresh_rate: 3000,
            value_size_default: 11,
            label_size_default: 10,
            combined_value_size_default: 10,
            monospace_values: false,
            cpu: CpuConfig::default(),
            cputemp: CpuTempConfig::default(),
            memory: MemoryConfig::default(),
            network1: NetworkConfig {
                variant: NetworkVariant::Combined,
                ..Default::default()
            },
            network2: NetworkConfig {
                variant: NetworkVariant::Upload,
                ..Default::default()
            },
            disks1: DisksConfig {
                variant: DisksVariant::Combined,
                ..Default::default()
            },
            disks2: DisksConfig {
                variant: DisksVariant::Read,
                ..Default::default()
            },
            gpus: HashMap::new(),
            sysmon: None,
            panel_spacing: 3,
            content_order: ContentOrder::default(),
        }
    }
}
