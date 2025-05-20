use std::env;

lazy_static::lazy_static! {
    pub static ref READ_BUF_CAPACITY: usize = get_env_usize(
        "NP_READ_BUF_CAPACITY",
        128 * 1024,
        "read buffer capacity"
    );
    pub static ref WRITE_BUF_CAPACITY: usize = get_env_usize(
        "NP_WRITE_BUF_CAPACITY",
        128 * 1024,
        "read buffer capacity"
    );
}

/// Gets a usize value from an environment variable or returns the default value.
/// Logs a message if a custom value is used or if parsing fails.
fn get_env_usize(env_var: &str, default: usize, description: &str) -> usize {
    env::var(env_var)
        .map(|val| match val.parse::<usize>() {
            Ok(size) => {
                info!(
                    "Using custom {}: {} (from environment variable {})",
                    description, size, env_var
                );
                size
            }
            Err(_) => {
                info!(
                    "Failed to parse {} value '{}', using default: {}B",
                    env_var, val, default
                );
                default
            }
        })
        .unwrap_or(default)
}
