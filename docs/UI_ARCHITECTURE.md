# UI Architecture

This document describes the architecture of RustIQ's UI module for V1.0, including how it uses [egui](https://docs.rs/egui/latest/egui/) for the graphical interface and integrates with the engine via message passing.

## V1.0 Scope

The V1.0 UI module implements:
- Receiving and processing `Event` messages from the engine
- Maintaining local application state derived from events
- Rendering a waterfall spectrogram display
- Displaying engine status (center frequency, sample rate, FFT size)

**Not in V1.0:**
- Sending `Command` messages (no user controls)
- Audio playback
- Interactive controls

## Technology Stack

### egui/eframe

[egui](https://github.com/emilk/egui) is an immediate mode GUI library for Rust that provides:
- **Immediate mode rendering**: UI is rebuilt every frame from current state
- **Cross-platform**: Native (via eframe) and web (via eframe + WASM)
- **No retained widget tree**: State lives in application code, not in UI framework
- **Simple API**: Minimal boilerplate, declarative-style code

[eframe](https://docs.rs/eframe/latest/eframe/) is the framework wrapper that provides:
- Application lifecycle management (startup, shutdown)
- Window creation and event loop
- Backend abstraction (native vs web)
- Integration with rendering backends (glow for OpenGL, wgpu for WebGPU)

### Why Immediate Mode?

Immediate mode is ideal for real-time signal visualization:
- State updates from engine events automatically trigger redraws
- No complex widget lifecycle management
- Natural fit for streaming data (waterfall continuously updates)
- Simple mental model: state → render → display

## Architecture Fit

The UI module is one endpoint of the bidirectional message protocol:

```
┌──────────────────────────────────────────┐
│         UI Module (Main Thread)          │
│  ┌────────────────────────────────────┐  │
│  │     eframe::App (RustIqApp)        │  │
│  │                                    │  │
│  │  ┌──────────────────────────────┐  │  │
│  │  │   Local State               │  │  │
│  │  │  - EngineState              │  │  │
│  │  │  - Waterfall history buffer │  │  │
│  │  └──────────────────────────────┘  │  │
│  │           ↑                        │  │
│  │  ┌────────┴─────────────────────┐  │  │
│  │  │   Event Processing Loop      │  │  │
│  │  │  (poll event_rx each frame)  │  │  │
│  │  └────────┬─────────────────────┘  │  │
│  │           ↓                        │  │
│  │  ┌──────────────────────────────┐  │  │
│  │  │   Rendering                  │  │  │
│  │  │  - Status display            │  │  │
│  │  │  - Waterfall widget          │  │  │
│  │  └──────────────────────────────┘  │  │
│  └────────────────────────────────────┘  │
└──────────┬──────────────────────┬─────────┘
           │ cmd_tx               │ event_rx
           │ (unused v1.0)        │ (flume)
           ↓                      ↑
┌──────────────────────────────────────────┐
│              Engine Thread               │
└──────────────────────────────────────────┘
```

### Module Boundary

The UI module exposes minimal public API:
- `ui::run()` function that takes channels and starts the app
- All internal components (widgets, state management) are private

Main's responsibility:
```rust
// In main.rs
use rustiq::{Engine, ui};

fn main() -> anyhow::Result<()> {
    let (cmd_tx, cmd_rx) = flume::unbounded();
    let (event_tx, event_rx) = flume::unbounded();

    // Create and spawn engine thread
    let engine = Engine::new(cmd_rx, event_tx);
    std::thread::spawn(move || {
        engine.run().expect("Engine failed");
    });

    // Run UI on main thread (blocking)
    ui::run(event_rx, cmd_tx)?;

    Ok(())
}
```

## Module Structure

```
src/ui/
├── mod.rs           # Public API: run() function, RustIqApp struct
├── state.rs         # UiState struct - local state management
└── waterfall.rs     # Waterfall rendering function
```

### Visibility

- **Public**: `ui::run()` function only
- **Private**: All internal types (RustIqApp, UiState, waterfall rendering)

## Data Flow

### Event Processing Pipeline

```
Engine Thread                Main Thread (UI)
─────────────                ────────────────

[SpectrumSink]
     │
     │ try_send()
     ↓
[event_tx] ═══════════════> [event_rx]
                                  │
                                  │ try_recv() (poll each frame)
                                  ↓
                            [Event handling]
                                  │
                    ┌─────────────┴──────────────┐
                    ↓                            ↓
            [StateSnapshot]              [SpectrumData]
                    │                            │
                    ↓                            ↓
          Update EngineState         Append to waterfall buffer
                    │                            │
                    └─────────────┬──────────────┘
                                  ↓
                            [egui renders]
                                  │
                                  ↓
                          [Display updates]
```

### Message Flow Details

1. **Engine produces events**: `SpectrumSink` calls `event_tx.try_send(Event::SpectrumData(vec))`
2. **UI polls for events**: Each frame, `RustIqApp::update()` calls `event_rx.try_recv()` in a loop
3. **State updates**: Events modify local `UiState`
4. **Rendering**: egui widgets read from `UiState` to draw UI

## State Management

### UiState Structure

```rust
// src/ui/state.rs

use std::collections::VecDeque;
use crate::messages::{Event, EngineState};

pub(super) struct UiState {
    /// Current engine state (from StateSnapshot)
    pub engine_state: Option<EngineState>,

    /// Waterfall history buffer
    /// Each entry is one FFT frame (Vec<f32>)
    /// Newest data at index 0, oldest at the end
    pub waterfall_history: VecDeque<Vec<f32>>,

    /// Maximum number of waterfall lines to keep
    pub waterfall_max_lines: usize,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            engine_state: None,
            waterfall_history: VecDeque::with_capacity(512),
            waterfall_max_lines: 512, // ~10 seconds at 50 FPS
        }
    }

    pub fn handle_event(&mut self, event: Event) {
        match event {
            Event::StateSnapshot(state) => {
                self.engine_state = Some(state);
            }
            Event::SpectrumData(data) => {
                // Add new data at front
                self.waterfall_history.push_front(data);

                // Trim old data from back
                while self.waterfall_history.len() > self.waterfall_max_lines {
                    self.waterfall_history.pop_back();
                }
            }
        }
    }
}
```

### State Update Timing

- **Event polling**: Non-blocking, drains all available events each frame
- **Frame rate**: ~60 FPS (egui default, depends on vsync)
- **Event rate**: Variable, depends on engine (FFT rate typically 20-100 Hz)
- **Decoupling**: Flume channel buffers events if UI is slower than engine

## Component Design

### RustIqApp (Main Application)

```rust
// src/ui/mod.rs

use crate::messages::{Command, Event};

pub struct RustIqApp {
    /// Receiver for events from engine
    event_rx: flume::Receiver<Event>,

    /// Sender for commands to engine
    cmd_tx: flume::Sender<Command>,

    /// Local application state
    state: UiState,
}

impl RustIqApp {
    fn new(
        event_rx: flume::Receiver<Event>,
        cmd_tx: flume::Sender<Command>,
    ) -> Self {
        Self {
            event_rx,
            cmd_tx,
            state: UiState::new(),
        }
    }
}

impl eframe::App for RustIqApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1. Process all pending events (non-blocking)
        while let Ok(event) = self.event_rx.try_recv() {
            self.state.handle_event(event);
        }

        // 2. Request continuous repaint (for streaming data)
        ctx.request_repaint();

        // 3. Render UI
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(engine_state) = &self.state.engine_state {
                // Display status
                ui.label(format!(
                    "Center: {} Hz | Rate: {} Hz | FFT: {}",
                    engine_state.center_frequency,
                    engine_state.sample_rate,
                    engine_state.fft_size
                ));

                ui.separator();

                // Render waterfall
                waterfall::render(ui, &self.state.waterfall_history, engine_state);
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("Waiting for engine connection...");
                });
            }
        });
    }
}
```

### Public Entry Point

```rust
// src/ui/mod.rs

pub fn run(
    event_rx: flume::Receiver<Event>,
    cmd_tx: flume::Sender<Command>,
) -> anyhow::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_title("RustIQ"),
        ..Default::default()
    };

    eframe::run_native(
        "RustIQ",
        options,
        Box::new(|_cc| Ok(Box::new(RustIqApp::new(event_rx, cmd_tx)))),
    )?;

    Ok(())
}
```

### Waterfall Widget

```rust
// src/ui/waterfall.rs

use egui::{Color32, ColorImage, TextureOptions, Ui};
use std::collections::VecDeque;
use crate::messages::EngineState;

pub(super) fn render(
    ui: &mut Ui,
    waterfall_history: &VecDeque<Vec<f32>>,
    engine_state: &EngineState,
) {
    if waterfall_history.is_empty() {
        ui.label("Waiting for spectrum data...");
        return;
    }

    let fft_size = engine_state.fft_size;
    let num_lines = waterfall_history.len();

    // Create image buffer
    let mut pixels = Vec::with_capacity(fft_size * num_lines);

    // Convert spectrum data to pixels
    for line in waterfall_history.iter() {
        for &magnitude in line.iter() {
            // Convert magnitude to color (grayscale)
            let intensity = magnitude_to_intensity(magnitude);
            let color = intensity_to_color(intensity);
            pixels.push(color);
        }
    }

    // Create texture
    let image = ColorImage {
        size: [fft_size, num_lines],
        pixels,
    };

    let texture = ui.ctx().load_texture(
        "waterfall",
        image,
        TextureOptions::LINEAR,
    );

    // Display image (stretched to fill available space)
    let available_size = ui.available_size();
    ui.image(&texture).fit_to_exact_size(available_size);
}

fn magnitude_to_intensity(magnitude: f32) -> f32 {
    // Convert linear magnitude to dB and normalize
    // Typical range: -80 dB to 0 dB
    let db = 20.0 * magnitude.log10();
    let min_db = -80.0;
    let max_db = 0.0;

    ((db - min_db) / (max_db - min_db)).clamp(0.0, 1.0)
}

fn intensity_to_color(intensity: f32) -> Color32 {
    // Simple grayscale mapping
    let value = (intensity * 255.0) as u8;
    Color32::from_gray(value)
}
```

## Event Handling

### Non-Blocking Poll Loop

The UI polls for events using `try_recv()` to avoid blocking the render thread:

```rust
impl eframe::App for RustIqApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process ALL available events before rendering
        // This ensures UI displays latest state even if multiple events queued
        while let Ok(event) = self.event_rx.try_recv() {
            self.state.handle_event(event);
        }

        // Continue with rendering...
    }
}
```

### Why Non-Blocking?

- **Responsive UI**: Never blocks main thread waiting for events
- **Catch-up behavior**: If UI is temporarily slow, it processes backlog on next frame
- **Graceful degradation**: Missing frames just means older data displayed briefly

### Continuous Repaint

For streaming data, request repaint every frame:

```rust
ctx.request_repaint();
```

This tells egui to immediately schedule another frame, creating continuous 60 FPS updates even when there's no user interaction.

## Threading Model

### Main Thread: UI Event Loop

The eframe event loop runs on the main thread and:
1. Handles OS events (mouse, keyboard, window events)
2. Polls engine events (non-blocking via `try_recv()`)
3. Updates local state based on received events
4. Renders UI using egui
5. Presents frame to display via OpenGL

This cycle repeats at ~60 FPS.

### Background Thread: Engine

The engine runs on a separate thread spawned by main:

```rust
std::thread::spawn(move || {
    engine.run().expect("Engine failed");
});
```

Engine and UI communicate only via channels - no shared memory.

### Why Main Thread for UI?

Most GUI frameworks (including eframe) require running on the main thread due to:
- OpenGL context restrictions
- macOS/iOS windowing requirements
- Event loop integration with OS

## Performance Considerations

### Texture Updates

Uploading texture data to GPU is the main performance cost:

**V1.0 approach:**
- Recreate texture every frame when there's new data
- Use `TextureOptions::LINEAR` for smooth scaling
- Acceptable for moderate FFT sizes (≤8192 bins)

### Memory Management

**Waterfall history buffer:**
- Use `VecDeque` for efficient push_front/pop_back
- Pre-allocate capacity (512 lines) to avoid reallocations
- Limit size to prevent unbounded memory growth
- Example memory usage: 4096 bins × 512 lines × 4 bytes ≈ 8 MB

**FFT data:**
- Each line is `Vec<f32>` with length = FFT size
- Memory per line: `fft_size * 4 bytes`
- 4096-bin FFT = 16 KB per frame

### Frame Rate vs Event Rate

Typical rates:
- UI frame rate: 60 FPS (16.7ms per frame)
- Engine FFT rate: 20-100 Hz (10-50ms per FFT)
- Event processing: <1ms (non-blocking poll)

The UI can comfortably handle 60+ FFT frames per second.

## Implementation Checklist

### Phase 1: Basic Structure
- [ ] Create `src/ui/mod.rs` with public `run()` function
- [ ] Implement `RustIqApp` struct with `eframe::App` trait
- [ ] Create `UiState` struct in `src/ui/state.rs`
- [ ] Implement event polling loop in `update()`

### Phase 2: Waterfall Display
- [ ] Create `src/ui/waterfall.rs`
- [ ] Implement `render()` function
- [ ] Convert spectrum data to texture
- [ ] Implement magnitude-to-grayscale conversion
- [ ] Display texture in UI

### Phase 3: Integration
- [ ] Update `main.rs` to call `ui::run()`
- [ ] Test with real engine output
- [ ] Verify waterfall displays 10 kHz spike from `SignalSource`

### Phase 4: Polish
- [ ] Add status display (frequency, sample rate, FFT size)
- [ ] Add "waiting for connection" message when no events yet
- [ ] Tune waterfall history buffer size if needed

## Key Takeaways

1. **Immediate mode UI simplifies state management** - no widget tree, just render from state
2. **Non-blocking event polling keeps UI responsive** - never blocks main thread
3. **VecDeque for waterfall history** - efficient FIFO buffer with bounded size
4. **Texture-based rendering** - convert spectrum data to GPU texture for display
5. **Public API is minimal** - only `ui::run()` exposed, all else private
6. **Testing is manual for V1.0** - visual verification of waterfall display

This architecture maintains clean separation from the engine while providing a responsive real-time display for SDR visualization.

## Resources

- [egui Documentation](https://docs.rs/egui/latest/egui/)
- [eframe Documentation](https://docs.rs/eframe/latest/eframe/)
- [egui GitHub](https://github.com/emilk/egui)
- [egui Web Demo](https://www.egui.rs/)

---

*This document will be updated as implementation progresses and design decisions are validated.*
