//! # Feature: Audio Transcription
//!
//! Whisper-powered transcription of audio attachments. Supports mp3, wav, m4a,
//! flac, ogg, aac, wma, mp4, mov, and avi formats.
//!
//! - **Version**: 1.1.0
//! - **Since**: 0.1.0
//! - **Toggleable**: true
//!
//! ## Changelog
//! - 1.1.0: Added configurable transcription modes (always/mention_only/disabled)
//! - 1.0.0: Initial release with Whisper API integration

use anyhow::Result;
use log::{error, info};
use std::process::Command;
use tokio::fs;

#[derive(Clone)]
pub struct AudioTranscriber {
    openai_api_key: String,
}

impl AudioTranscriber {
    pub fn new(openai_api_key: String) -> Self {
        AudioTranscriber { openai_api_key }
    }

    pub async fn transcribe_file(&self, file_path: &str) -> Result<String> {
        info!("Transcribing audio file: {file_path}");

        if !self.is_audio_file(file_path) {
            return Err(anyhow::anyhow!("File is not a supported audio format"));
        }

        if fs::metadata(file_path).await.is_err() {
            return Err(anyhow::anyhow!("Audio file not found: {}", file_path));
        }

        let output = Command::new("curl")
            .args([
                "https://api.openai.com/v1/audio/transcriptions",
                "-H", &format!("Authorization: Bearer {}", self.openai_api_key),
                "-H", "Content-Type: multipart/form-data",
                "-F", &format!("file=@{file_path}"),
                "-F", "model=whisper-1",
            ])
            .output()?;

        if output.status.success() {
            let response = String::from_utf8(output.stdout)?;
            let json: serde_json::Value = serde_json::from_str(&response)?;
            
            if let Some(text) = json.get("text").and_then(|t| t.as_str()) {
                info!("Transcription successful, length: {} characters", text.len());
                Ok(text.to_string())
            } else if let Some(error) = json.get("error") {
                error!("OpenAI API error: {error}");
                Err(anyhow::anyhow!("OpenAI API error: {}", error))
            } else {
                error!("Unexpected response format: {response}");
                Err(anyhow::anyhow!("Unexpected response format"))
            }
        } else {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            error!("Transcription failed: {error_msg}");
            Err(anyhow::anyhow!("Transcription failed: {}", error_msg))
        }
    }

    fn is_audio_file(&self, file_path: &str) -> bool {
        let audio_extensions = [
            ".mp3", ".wav", ".m4a", ".flac", ".ogg", ".aac", ".wma", ".mp4", ".mov", ".avi"
        ];
        
        let file_path_lower = file_path.to_lowercase();
        audio_extensions.iter().any(|ext| file_path_lower.ends_with(ext))
    }

    pub async fn download_and_transcribe_attachment(&self, url: &str, filename: &str) -> Result<String> {
        let temp_file = format!("/tmp/discord_audio_{filename}");
        
        info!("Downloading audio attachment: {filename}");
        
        let output = Command::new("curl")
            .args(["-o", &temp_file, url])
            .output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Failed to download audio file"));
        }

        let transcription = self.transcribe_file(&temp_file).await;
        
        if let Err(e) = fs::remove_file(&temp_file).await {
            error!("Failed to cleanup temp file {temp_file}: {e}");
        }

        transcription
    }
}