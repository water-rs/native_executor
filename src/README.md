# Native Executor Source Code

This directory contains the core implementation of the native-executor library.

## Architecture Overview

The Native Executor provides a high-performance, platform-native asynchronous task executor that leverages operating system primitives for optimal performance. The library uses a trait-based design with platform-specific implementations to provide zero-cost abstractions over native OS APIs.

## Module Structure

### `lib.rs`
Core library entry point containing:
- `Task<T>` - Thread-safe async task handles with priority control
- `LocalTask<T>` - Thread-local task handles for non-Send futures
- `Priority` - Task scheduling priority levels (Default, Background)
- Main API functions (`task`, `exec_main`, etc.)

### `apple.rs` 
Apple platform implementation using Grand Central Dispatch:
- `ApplePlatformExecutor` - GCD-based task scheduling
- Priority mapping to GCD queue priorities
- Main thread and background execution with optimal system integration

### `timer.rs`
High-precision timer implementation:
- `Timer` - Platform-native timing futures with zero busy-waiting
- `sleep()` - Convenience function for second-based delays
- Leverages OS scheduling for accurate timing

### `local_value.rs`
Thread-safety utilities:
- `LocalValue<T>` - Thread-local value containers with runtime checks
- `OnceValue<T>` - Single-consumption value wrappers
- Thread affinity enforcement with fail-fast safety

### `main_value.rs`
Main-thread access patterns:
- `MainValue<T>` - Cross-thread safe main-thread value access
- Async main-thread operation scheduling
- Safe UI and API access from any thread context

### `main.rs`
Example binary demonstrating basic usage patterns.

## Platform Abstraction

The library uses a trait-based design for platform independence:

```rust
trait PlatformExecutor {
    fn exec_main(f: impl FnOnce() + Send + 'static);
    fn exec(f: impl FnOnce() + Send + 'static, priority: Priority);
    fn exec_after(delay: Duration, f: impl FnOnce() + Send + 'static);
}
```

### Current Implementation
- **Apple Platforms**: Full GCD integration via `ApplePlatformExecutor`
  - Maps to system thread pools and dispatch queues
  - Priority levels mapped to GCD queue priorities
  - Main thread execution via `dispatch_async_f(dispatch_get_main_queue())`

### Planned Implementations
- **Windows**: Thread pool APIs and completion ports
- **Linux**: epoll/io_uring based implementation  
- **Android**: Android-specific optimizations
- **WebAssembly**: Browser and WASI compatibility

## Key Design Principles

1. **Zero-cost abstractions**: Minimal overhead over direct OS API usage
2. **Platform-native performance**: Leverage OS primitives for optimal scheduling
3. **Memory safety**: Compile-time and runtime safety guarantees
4. **Thread-safety**: Safe cross-thread communication patterns
5. **Fail-fast safety**: Runtime checks with clear panic messages

## Dependencies

- `async-task`: Task spawning and management
- `executor-core`: Core executor trait definitions  
- `futures-lite`: Future utilities and combinators
- `spin`: Lock-free atomic operations
- `dispatch`: Apple GCD bindings (Apple platforms only)

The implementation prioritizes minimal dependencies while maximizing platform integration and performance.
