pub mod control_flow;
pub mod data_sources;
pub mod env;
pub mod extraction;
pub mod goose;
pub mod interpolation;
pub mod macros;
pub mod pacing;
pub mod reporting;
pub mod sla;

pub fn init_logging() {
    tracing_subscriber::fmt::init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_logging() {
        // Just verify it doesn't panic
        init_logging();
    }
}
