const fn unwrap_or_cargo_version(opt: Option<&'static str>) -> &'static str {
    match opt {
        Some(val) => val,
        None => env!("CARGO_PKG_VERSION"),
    }
}

pub const VERSION: &str = unwrap_or_cargo_version(option_env!("APP_VERSION"));