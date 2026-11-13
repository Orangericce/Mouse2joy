use evdev::{Device, EventType, AbsoluteAxisCode};

fn print_caps(path: &str) {
    if let Ok(device) = Device::open(path) {
        let caps = device.supported_events();
        println!("=== {} ({}) ===", path, device.name().unwrap_or("unknown"));
        println!("  RELATIVE: {}", caps.contains(EventType::RELATIVE));
        println!("  ABSOLUTE: {}", caps.contains(EventType::ABSOLUTE));
        println!("  KEY: {}", caps.contains(EventType::KEY));
        
        if caps.contains(EventType::RELATIVE) {
            if let Some(axes) = device.supported_relative_axes() {
                println!("  REL axes: {:?}", axes);
            }
        }
        if caps.contains(EventType::ABSOLUTE) {
            if let Some(axes) = device.supported_absolute_axes() {
                println!("  ABS axes: {:?}", axes);
            }
        }
        if caps.contains(EventType::KEY) {
            if let Some(keys) = device.supported_keys() {
                println!("  Keys (first 20): {:?}", keys.iter().take(20).collect::<Vec<_>>());
            }
        }
    } else {
        println!("=== {} ===", path);
        println!("  Failed to open");
    }
}

fn main() {
    for i in 0..18 {
        let path = format!("/dev/input/event{}", i);
        if std::path::Path::new(&path).exists() {
            print_caps(&path);
        }
    }
}
