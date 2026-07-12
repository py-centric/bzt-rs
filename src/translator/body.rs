use crate::engine::interpolation::Interpolator;
use crate::engine::macros::MacroEvaluator;
use crate::engine::BztError;
use std::collections::HashMap;

/// Resolves the request body from either a file or inline definition.
///
/// Priority: `body_file` > inline `body`. When a file is specified, the path
/// is validated for security (no directory traversal) and the file size is
/// checked before reading. The resulting content is interpolated with session
/// variables and evaluated for macro expressions.
///
/// # Errors
///
/// Returns `BztError` if path validation, security checks, or macro
/// evaluation fails.
pub(crate) async fn resolve_body(
    body: Option<&str>,
    body_file: Option<&str>,
    variables: &HashMap<String, String>,
    record: Option<&HashMap<String, String>>,
) -> Result<String, BztError> {
    let raw = if let Some(bf) = body_file {
        let body_path = MacroEvaluator::evaluate(bf, record)?;
        let canonical = crate::engine::data_sources::validate_path(&body_path)?;
        crate::engine::data_sources::check_file_size(&canonical)?;
        match tokio::fs::read_to_string(&canonical).await {
            Ok(content) => content,
            Err(e) => {
                tracing::warn!(
                    "[BODY_FILE] Failed to read body file '{}': {}",
                    canonical.display(),
                    e
                );
                String::new()
            }
        }
    } else {
        body.map(|b| Interpolator::interpolate(b, variables))
            .unwrap_or_default()
    };

    let final_body = MacroEvaluator::evaluate(&raw, record)?;
    Ok(final_body)
}
