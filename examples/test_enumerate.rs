use evdev::{Device, EventType, RelativeAxisCode};

fn main() {
    for i in 1..9 {
        let path = format!("/dev/input/event{}", i);
        if let Ok(mut device) = Device::open(&path) {
            let caps = device.supported_events();
            println!("=== {} {} ===", i, path);
            println!("  has_rel: {}, has_keys: {}", 
                     caps.contains(EventType::RELATIVE),
                     caps.contains(EventType::KEY));
            if let Some(axes) = device.supported_relative_axes() {
                println!("  REL axes: {:?}", axes);
            }
        }
    }
}
