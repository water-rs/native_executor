# Native Executor

[![Crates.io](https://img.shields.io/crates/v/native-executor.svg)](https://crates.io/crates/native-executor)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![docs.rs](https://docs.rs/native-executor/badge.svg)](https://docs.rs/native-executor)

Platform-native async task executor that leverages OS event loops (GCD, GDK) for optimal performance.

## Features

- **Platform-native scheduling**: Direct GCD integration on Apple platforms
- **Priority-aware execution**: Background vs default task prioritization
- **Thread-local safety**: Non-Send future execution with compile-time guarantees
- **High-precision timers**: OS-native timing without busy-waiting
- **Thread-safe utilities**: `LocalValue`, `OnceValue`, `MainValue` containers
- **Zero-cost abstractions**: Direct OS API usage, no additional runtime

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
native-executor = "0.2.0"
```

## Quick Start

```rust
use native_executor::{spawn_local, timer::Timer};
use std::time::Duration;

// Spawn a task with default priority
let handle = spawn_local(async {
    println!("Starting async task");

    // High-precision timer using platform-native scheduling
    Timer::after(Duration::from_secs(1)).await;

    println!("Task completed after 1 second");
});

// Keep the main thread alive to allow tasks to complete
std::thread::sleep(Duration::from_secs(2));
```

## Core Components

### Task Spawning

```rust
use native_executor::{spawn, spawn_local, spawn_main, spawn_with_priority, Priority};

spawn(async { /* default priority */ });
spawn_local(async { /* non-Send, main thread */ });
spawn_main(async { /* Send, main thread */ });
spawn_with_priority(async { /* background work */ }, Priority::Background);
```

### Timers

```rust
use native_executor::timer::{Timer, sleep};
use std::time::Duration;

# async {
Timer::after(Duration::from_millis(100)).await;  // Precise timing
Timer::after_secs(2).await;                      // Convenience method
sleep(1).await;                                  // Simple sleep
# };
```

### Thread-Safe Containers

```rust
use native_executor::{LocalValue, OnceValue, MainValue};

// Thread-local access only
let local = LocalValue::new(42);
assert_eq!(*local, 42);

// Single-consumption semantics
let once = OnceValue::new("consume once");
let value = once.take();

// Cross-thread with main-thread execution
let main_val = MainValue::new(String::from("UI data"));
# async {
let len = main_val.handle(|s| s.len()).await;
# };
```

## Platform Support

**Current**: Apple platforms (macOS, iOS, tvOS, watchOS) via Grand Central Dispatch\
**Planned**: Linux (GDK), Windows (IOCP), Android (Looper), WebAssembly

Unsupported platforms fail at compile-time with clear error messages.

## Examples

```bash
cargo run --example simple_task    # Basic spawning
cargo run --example priority       # Priority control
cargo run --example timers         # High-precision timing
cargo run --example main_thread     # Main thread execution
cargo run --example local_value     # Thread-safe containers
```

## License

This project is licensed under the MIT License.
