use native_executor::{LocalValue, OnceValue};
use std::time::Duration;

#[allow(clippy::uninlined_format_args)]
fn main() {
    // Create a thread-local value
    let local = LocalValue::new(42);
    
    // Access it on the same thread
    println!("Value: {:?}", local);
    println!("Dereferenced value: {}", *local);
    
    // Create a once-value
    let once = OnceValue::new("take me once");
    
    // Take the value
    let value = once.take();
    println!("Taken value: {value}");
    
    // Keep the main thread alive
    std::thread::sleep(Duration::from_secs(1));
}
