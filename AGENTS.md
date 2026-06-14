# Sysmon COSMIC Applet — AGENTS.md
****
## Project identity

- **Binary**: `cosmic-ext-applet-sysmon` (package name `cosmic-ext-applet-sysmon` in `Cargo.toml`)
- **App ID**: `io.github.cosmic_utils.sysmon-applet`
- **Config namespace**: `io.github.cosmic_utils.sysmon-applet` (COSMIC `cosmic_config`, not flat files)
- **License**: GPL-3.0-only

## Build & commands

Use `just`, not raw `cargo`. Everything is in the justfile.

| Command                       | What it does                                                                  |
| ----------------------------- | ----------------------------------------------------------------------------- |
| `just` / `just build-release` | Release build                                                                 |
| `just build-debug`            | Debug build                                                                   |
| `just run`                    | Run with `RUST_LOG=cosmic_tasks=info RUST_BACKTRACE=full cargo run --release` |
| `just dev`                    | `cargo fmt` then `just run`                                                   |
| `just check`                  | `cargo clippy --all-features -- -W clippy::pedantic`                          |
| `just check-json`             | Same but JSON output                                                          |
| `just install`                | Strips & installs binary + desktop + icons + metainfo into `/usr`             |
| `just deb` / `just rpm`       | Build packages after `just build-release`                                     |
| `just clean`                  | `cargo clean`                                                                 |

## Logging

```sh
journalctl SYSLOG_IDENTIFIER=cosmic-ext-applet-sysmon
```

- Release builds: systemd journal with fallback to stdout.
- Debug builds: stdout via fern.
- The binary name for journald is `cosmic-ext-applet-sysmon`.

## Code quirks

- **`lyon_charts` feature**: Gated behind `#[cfg(feature = "lyon_charts")]` / `mod charts`. Disabled by default. Adds canvas-based chart rendering.
- **`make_config!` macro** in `config.rs`: Generates config structs with `chart_visible`, `value_visible`, `label_visible`, `icon_visible` fields + helper methods.
- **Localization**: Uses `fl!()` macro and `i18n-embed` with Fluent files in `i18n/` (14 locales). Fallback: `en`.
- **GPU detection**: Supports Nvidia (via `nvml-wrapper`), AMD, and Intel. Nvidia detection retries up to 5 times during init because the NVML runtime may be slow.

## Src layout

```
src/main.rs          → Entrypoint, logger setup, calls cosmic::applet::run::<Sysmon>
src/app.rs           → Main Application impl (Sysmon struct, Message enum, view/update)
src/config.rs        → SysmonConfig + all sub-config structs via make_config! macro
src/i18n.rs          → fl!() macro, i18n-embed wiring
src/sensors/         → Sensor trait + cpu, cputemp, memory, network, disks, gpu mods
src/sensors/gpu/     → amd.rs, intel.rs, nvidia.rs (per-GPU-backend logic)
src/sensors/gpus.rs  → Gpus collection (HashMap<String, Gpu>), manages multiple GPUs
src/barchart.rs      → StackedBarSvg rendering
src/svg_graph.rs     → SVG-based graph rendering
src/colorpicker.rs   → Color picker dialog
src/charts/          → lyon_charts feature: heat, line, ring renderers
```

## Flatpak

- Manifest: `io.github.cosmic_utils.sysmon-applet.json`
- Cargo sources: generated via `just flatpak-cargo-sources` (runs `flatpak-cargo-generator.py`)
- Build: `just flatpak-builder` or manually via the manifest.
- Deployed to `org.freedesktop.Platform//25.08` runtime.
- Finish-args grant Wayland, `rw` cosmic config, `ro` hwmon/drm, and `device=all`.

## Test / CI

No test suite present. No CI workflows beyond `FUNDING.YML`. No pre-commit hooks.
