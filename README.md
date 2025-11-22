# Native Executor

[![Crates.io](https://img.shields.io/crates/v/native-executor.svg)](https://crates.io/crates/native-executor)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![docs.rs](https://docs.rs/native-executor/badge.svg)](https://docs.rs/native-executor)

Platform-native async task executor that leverages OS event loops (GCD, GDK) for optimal performance.

## Features

- **Platform-native scheduling**: Direct GCD integration on Apple platforms
- **Priority-aware execution**: Background vs default task prioritization
- **Thread-local safety**: Non-Send future execution with compile-time guarantees
- **Mailbox-based messaging**: Share state via serialized cross-thread queues
- **Zero-cost abstractions**: Direct OS API usage, no additional runtime

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
native-executor = "0.5"
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

### Mailbox Messaging

```rust
use native_executor::mailbox::Mailbox;
use std::{cell::RefCell, collections::HashMap};

let mailbox = Mailbox::main(RefCell::new(HashMap::<String, i32>::new()));

// Send fire-and-forget updates
mailbox.handle(|map| {
    map.borrow_mut().insert("key".to_string(), 42);
});

# async fn docs() {
let mailbox = Mailbox::main(RefCell::new(HashMap::<String, i32>::new()));

mailbox.handle(|map| {
    map.borrow_mut().insert("key".to_string(), 7);
});

// Await a response from the mailbox task
let value = mailbox.call(|map| {
    map.borrow().get("key").copied().unwrap_or_default()
}).await;
assert_eq!(value, 7);
# }
```

## Platform Support

**Current**: Apple platforms (macOS, iOS, tvOS, watchOS) via Grand Central Dispatch, Android (native worker queues)\
**Planned**: Linux (GDK)

Unsupported platforms fail at compile-time with clear error messages.

## Polyfill Feature

The optional `polyfill` feature (enabled by default) provides a simulated
executor for targets without a native implementation. Its behavior is as
follows:

- On Apple, Android, and `wasm32` targets the feature is a no-op – the native
  executors and timers always take precedence.
- On other targets the crate will not build unless the `polyfill` feature is
  enabled. Disabling it makes the lack of a native executor a hard error.
- The polyfill spins up its own worker threads and exposes a synthetic
  "main thread". Call `native_executor::polyfill::start_main_executor()` on a
  dedicated thread before using `spawn_main` or `spawn_local`.
- Because this main thread is not provided by the OS event loop, code that
  depends on true main-thread semantics (UI frameworks, platform APIs, etc.)
  may behave differently. The feature exists only as a portability fallback.

Example setup for unsupported targets:

```rust
#[cfg(all(feature = "polyfill", not(any(target_vendor = "apple", target_arch = "wasm32", target_os = "android"))))]
std::thread::spawn(|| native_executor::polyfill::start_main_executor());
```

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
