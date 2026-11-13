use evdev::{Device, EventType, AbsoluteAxisCode};

fn is_mouse_device(path: &str) -> bool {
    if let Ok(device) = Device::open(path) {
        let caps = device.supported_events();
        let has_rel = caps.contains(EventType::RELATIVE);
        let has_abs = caps.contains(EventType::ABSOLUTE);
        let has_keys = caps.contains(EventType::KEY);
        
        let is_mouse = has_rel && !has_keys;
        let is_touchpad = has_abs && has_keys;
        
        if is_touchpad {
            if let Some(axes) = device.supported_absolute_axes() {
                let has_x = axes.contains(AbsoluteAxisCode::ABS_X);
                let has_y = axes.contains(AbsoluteAxisCode::ABS_Y);
                if has_x && has_y {
                    return true;
                }
            }
        }
        
        is_mouse || is_touchpad
    } else {
        false
    }
}

fn main() {
    for i in 0..18 {
        let path = format!("/dev/input/event{}", i);
        if std::path::Path::new(&path).exists() {
            let result = is_mouse_device(&path);
            if result {
                println!("{} -> MOUSE/TRACKPAD", path);
            } else {
                println!("{} -> NOT a mouse/trackpad", path);
            }
        }
    }
}
