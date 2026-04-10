/// The version of the harness-cli package, read from Cargo.toml at compile time.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The package name, read from Cargo.toml at compile time.
pub const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");
