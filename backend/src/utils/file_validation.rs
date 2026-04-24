use crate::errors::{AppError, Result};

/// Validates that the file extension and MIME type are supported audio formats
///
/// # Arguments
///
/// * `file_name` - The original filename from the upload
/// * `content_type` - The MIME type from the multipart field
///
/// # Returns
///
/// * `Result<String>` - Ok(extension) if valid, Err(AppError) if invalid
///
/// # Supported formats
///
/// MP3, WAV, FLAC, OGG, M4A, AAC
pub fn validate_audio_file(
    file_name: Option<&String>,
    content_type: Option<&String>,
) -> Result<String> {
    // Extract file extension
    let ext = file_name
        .and_then(|f: &String| f.rsplit('.').next())
        .map(|s| s.to_lowercase())
        .unwrap_or_else(|| "bin".to_string());

    let allowed_extensions = ["mp3", "wav", "flac", "ogg", "m4a", "aac", "webm"];
    let is_valid_ext = allowed_extensions.contains(&ext.as_str());

    // Validate MIME type
    let is_valid_mime = content_type.as_ref().is_some_and(|mime| {
        let mime = mime.to_lowercase();
        mime.starts_with("audio/") || mime == "application/ogg" || mime == "video/mp4" // M4A is technically a subset of MP4 container
    });

    if !is_valid_ext || !is_valid_mime {
        tracing::error!(
            "Validation failed: Unsupported file type. ext={:?}, mime={:?}",
            ext,
            content_type
        );
        return Err(AppError::Validation(format!(
            "Unsupported file type: {}. Please upload a supported audio file (MP3, WAV, FLAC, OGG, M4A AAC).",
            ext
        )));
    }

    Ok(ext)
}
