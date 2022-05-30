use std::env;

pub fn truecolor_available() -> bool {
    if let Ok(value) = env::var("COLORTERM") {
        value.contains("truecolor") || value.contains("24bit")
    } else {
        false
    }
}
