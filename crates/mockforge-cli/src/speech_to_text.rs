//! Speech-to-Text (STT) module for voice input
//!
//! This module provides speech-to-text functionality for the CLI voice interface.
//! It supports multiple backends:
//! - Text input (fallback, always available)
//! - Cloud APIs (OpenAI Whisper, Google Speech-to-Text) - optional via `stt-cloud` feature
//! - Offline STT (vosk-rs) - optional via `stt-vosk` feature
//!
//! The module gracefully falls back to text input if STT is not available.

use std::fmt;
use std::io::{self, Write};
use std::path::PathBuf;

/// Speech-to-text errors
#[derive(Debug)]
pub enum SttError {
    NotAvailable(String),
    AudioCapture(String),
    Transcription(String),
    Io(io::Error),
}

impl fmt::Display for SttError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SttError::NotAvailable(msg) => write!(f, "STT not available: {}", msg),
            SttError::AudioCapture(msg) => write!(f, "Audio capture error: {}", msg),
            SttError::Transcription(msg) => write!(f, "Transcription error: {}", msg),
            SttError::Io(err) => write!(f, "IO error: {}", err),
        }
    }
}

impl std::error::Error for SttError {}

impl From<io::Error> for SttError {
    fn from(err: io::Error) -> Self {
        SttError::Io(err)
    }
}

/// Speech-to-text backend trait
pub trait SpeechToTextBackend: Send + Sync {
    /// Check if this backend is available
    fn is_available(&self) -> bool;

    /// Transcribe audio from microphone
    fn transcribe(&self) -> Result<String, SttError>;

    /// Get the name of this backend
    fn name(&self) -> &'static str;

    /// Get a description of the backend
    fn description(&self) -> &'static str {
        self.name()
    }
}

/// Text input backend (always available, fallback)
pub struct TextInputBackend;

impl SpeechToTextBackend for TextInputBackend {
    fn is_available(&self) -> bool {
        true // Always available
    }

    fn transcribe(&self) -> Result<String, SttError> {
        // Prompt user for text input
        print!("📝 Enter your command: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        Ok(input.trim().to_string())
    }

    fn name(&self) -> &'static str {
        "text-input"
    }
}

/// Cloud API backend configuration
#[derive(Debug, Clone)]
pub enum CloudSttProvider {
    /// OpenAI Whisper API
    OpenAiWhisper,
    /// Google Cloud Speech-to-Text
    GoogleSpeech,
}

impl CloudSttProvider {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "openai" | "whisper" | "openai-whisper" => Some(Self::OpenAiWhisper),
            "google" | "google-speech" | "gcp" => Some(Self::GoogleSpeech),
            _ => None,
        }
    }
}

/// OpenAI Whisper API backend
#[cfg(feature = "stt-cloud")]
pub struct OpenAiWhisperBackend {
    api_key: String,
    client: reqwest::Client,
}

#[cfg(feature = "stt-cloud")]
impl OpenAiWhisperBackend {
    pub fn new(api_key: Option<String>) -> Result<Self, SttError> {
        let api_key =
            api_key.or_else(|| std::env::var("OPENAI_API_KEY").ok()).ok_or_else(|| {
                SttError::NotAvailable(
                    "OpenAI API key not found. Set OPENAI_API_KEY environment variable."
                        .to_string(),
                )
            })?;

        Ok(Self {
            api_key,
            client: reqwest::Client::new(),
        })
    }

    async fn capture_and_transcribe(&self) -> Result<String, SttError> {
        // For CLI, we'll use a simple approach: record to temp file then upload
        // In a production implementation, you'd want streaming audio capture
        println!("🎤 Recording audio (press Enter to stop)...");

        // For now, we'll use a workaround: prompt user to provide audio file
        // In a full implementation, we'd use cpal to capture from microphone
        println!("⚠️  Audio capture from microphone requires additional setup.");
        println!("   For now, please provide the path to an audio file (WAV, MP3, etc.):");
        print!("   Audio file path: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let audio_path = input.trim();

        if audio_path.is_empty() {
            return Err(SttError::AudioCapture("No audio file provided".to_string()));
        }

        let audio_path = PathBuf::from(audio_path);
        if !audio_path.exists() {
            return Err(SttError::AudioCapture(format!(
                "Audio file not found: {}",
                audio_path.display()
            )));
        }

        // Read audio file
        let audio_data = std::fs::read(&audio_path)
            .map_err(|e| SttError::AudioCapture(format!("Failed to read audio file: {}", e)))?;

        // Upload to OpenAI Whisper API
        self.transcribe_audio(&audio_data).await
    }

    async fn transcribe_audio(&self, audio_data: &[u8]) -> Result<String, SttError> {
        use reqwest::multipart;

        // Create multipart form
        let file_part = multipart::Part::bytes(audio_data.to_vec())
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| SttError::Transcription(format!("Failed to create form part: {}", e)))?;

        let form = multipart::Form::new().part("file", file_part).text("model", "whisper-1");

        let response = self
            .client
            .post("https://api.openai.com/v1/audio/transcriptions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .send()
            .await
            .map_err(|e| SttError::Transcription(format!("API request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SttError::Transcription(format!("OpenAI API error: {}", error_text)));
        }

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| SttError::Transcription(format!("Failed to parse response: {}", e)))?;

        result["text"]
            .as_str()
            .ok_or_else(|| SttError::Transcription("Invalid response format".to_string()))
            .map(|s| s.to_string())
    }
}

#[cfg(feature = "stt-cloud")]
impl SpeechToTextBackend for OpenAiWhisperBackend {
    fn is_available(&self) -> bool {
        true // If we can create it, it's available
    }

    fn transcribe(&self) -> Result<String, SttError> {
        // Use tokio runtime for async
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| SttError::Transcription(format!("Failed to create runtime: {}", e)))?;
        rt.block_on(self.capture_and_transcribe())
    }

    fn name(&self) -> &'static str {
        "openai-whisper"
    }

    fn description(&self) -> &'static str {
        "OpenAI Whisper API (cloud-based, high accuracy)"
    }
}

/// Google Cloud Speech-to-Text backend
#[cfg(feature = "stt-cloud")]
pub struct GoogleSpeechBackend {
    api_key: String,
    client: reqwest::Client,
}

#[cfg(feature = "stt-cloud")]
impl GoogleSpeechBackend {
    pub fn new(api_key: Option<String>) -> Result<Self, SttError> {
        let api_key = api_key
            .or_else(|| std::env::var("GOOGLE_CLOUD_API_KEY").ok())
            .or_else(|| std::env::var("GOOGLE_APPLICATION_CREDENTIALS").ok())
            .ok_or_else(|| {
                SttError::NotAvailable(
                    "Google Cloud API key not found. Set GOOGLE_CLOUD_API_KEY or GOOGLE_APPLICATION_CREDENTIALS environment variable.".to_string()
                )
            })?;

        Ok(Self {
            api_key,
            client: reqwest::Client::new(),
        })
    }

    async fn capture_and_transcribe(&self) -> Result<String, SttError> {
        println!("🎤 Recording audio (press Enter to stop)...");
        println!("⚠️  Audio capture from microphone requires additional setup.");
        println!("   For now, please provide the path to an audio file (WAV, MP3, etc.):");
        print!("   Audio file path: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let audio_path = input.trim();

        if audio_path.is_empty() {
            return Err(SttError::AudioCapture("No audio file provided".to_string()));
        }

        let audio_path = PathBuf::from(audio_path);
        if !audio_path.exists() {
            return Err(SttError::AudioCapture(format!(
                "Audio file not found: {}",
                audio_path.display()
            )));
        }

        let audio_data = std::fs::read(&audio_path)
            .map_err(|e| SttError::AudioCapture(format!("Failed to read audio file: {}", e)))?;

        self.transcribe_audio(&audio_data).await
    }

    async fn transcribe_audio(&self, audio_data: &[u8]) -> Result<String, SttError> {
        // Base64 encode audio
        let audio_base64 = base64::encode(audio_data);

        let request_body = serde_json::json!({
            "config": {
                "encoding": "LINEAR16",
                "sampleRateHertz": 16000,
                "languageCode": "en-US",
                "enableAutomaticPunctuation": true
            },
            "audio": {
                "content": audio_base64
            }
        });

        let url = format!("https://speech.googleapis.com/v1/speech:recognize?key={}", self.api_key);

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| SttError::Transcription(format!("API request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(SttError::Transcription(format!(
                "Google Speech API error: {}",
                error_text
            )));
        }

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| SttError::Transcription(format!("Failed to parse response: {}", e)))?;

        // Extract transcript from Google's response format
        if let Some(results) = result["results"].as_array() {
            if let Some(first_result) = results.first() {
                if let Some(alternatives) = first_result["alternatives"].as_array() {
                    if let Some(first_alt) = alternatives.first() {
                        if let Some(transcript) = first_alt["transcript"].as_str() {
                            return Ok(transcript.to_string());
                        }
                    }
                }
            }
        }

        Err(SttError::Transcription("No transcript found in response".to_string()))
    }
}

#[cfg(feature = "stt-cloud")]
impl SpeechToTextBackend for GoogleSpeechBackend {
    fn is_available(&self) -> bool {
        true
    }

    fn transcribe(&self) -> Result<String, SttError> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| SttError::Transcription(format!("Failed to create runtime: {}", e)))?;
        rt.block_on(self.capture_and_transcribe())
    }

    fn name(&self) -> &'static str {
        "google-speech"
    }

    fn description(&self) -> &'static str {
        "Google Cloud Speech-to-Text API (cloud-based, high accuracy)"
    }
}

/// Cloud API backend (generic wrapper)
pub struct CloudApiBackend {
    provider: String,
    api_key: Option<String>,
}

impl CloudApiBackend {
    pub fn new(provider: String, api_key: Option<String>) -> Self {
        Self { provider, api_key }
    }
}

impl SpeechToTextBackend for CloudApiBackend {
    fn is_available(&self) -> bool {
        #[cfg(feature = "stt-cloud")]
        {
            // Check if we can create a backend for this provider
            match CloudSttProvider::from_str(&self.provider) {
                Some(CloudSttProvider::OpenAiWhisper) => {
                    OpenAiWhisperBackend::new(self.api_key.clone()).is_ok()
                }
                Some(CloudSttProvider::GoogleSpeech) => {
                    GoogleSpeechBackend::new(self.api_key.clone()).is_ok()
                }
                None => false,
            }
        }
        #[cfg(not(feature = "stt-cloud"))]
        {
            false
        }
    }

    fn transcribe(&self) -> Result<String, SttError> {
        #[cfg(feature = "stt-cloud")]
        {
            match CloudSttProvider::from_str(&self.provider) {
                Some(CloudSttProvider::OpenAiWhisper) => {
                    let backend = OpenAiWhisperBackend::new(self.api_key.clone())?;
                    backend.transcribe()
                }
                Some(CloudSttProvider::GoogleSpeech) => {
                    let backend = GoogleSpeechBackend::new(self.api_key.clone())?;
                    backend.transcribe()
                }
                None => Err(SttError::NotAvailable(format!(
                    "Unknown cloud provider: {}",
                    self.provider
                ))),
            }
        }
        #[cfg(not(feature = "stt-cloud"))]
        {
            Err(SttError::NotAvailable(
                "Cloud STT feature not enabled. Enable with --features stt-cloud".to_string(),
            ))
        }
    }

    fn name(&self) -> &'static str {
        "cloud-api"
    }
}

/// Vosk offline STT backend
#[cfg(feature = "stt-vosk")]
pub struct VoskBackend {
    model_path: Option<PathBuf>,
    sample_rate: f32,
}

#[cfg(feature = "stt-vosk")]
impl VoskBackend {
    pub fn new(model_path: Option<PathBuf>) -> Result<Self, SttError> {
        // Try to find model in common locations if not provided
        let model_path = if let Some(path) = model_path {
            if path.exists() {
                Some(path)
            } else {
                return Err(SttError::NotAvailable(format!(
                    "Vosk model not found at: {}",
                    path.display()
                )));
            }
        } else {
            // Try common locations
            let common_paths = vec![
                PathBuf::from("./vosk-model"),
                PathBuf::from("~/.vosk/models"),
                PathBuf::from("/usr/share/vosk/models"),
            ];

            common_paths.into_iter().find(|p| p.exists()).or_else(|| {
                std::env::var("VOSK_MODEL_PATH").ok().map(PathBuf::from).filter(|p| p.exists())
            })
        };

        if model_path.is_none() {
            return Err(SttError::NotAvailable(
                "Vosk model not found. Set VOSK_MODEL_PATH environment variable or download a model from https://alphacephei.com/vosk/models".to_string()
            ));
        }

        Ok(Self {
            model_path,
            sample_rate: 16000.0, // Standard sample rate for Vosk
        })
    }

    fn transcribe_audio_file(&self, audio_path: &PathBuf) -> Result<String, SttError> {
        use hound::WavReader;
        use vosk::{Model, Recognizer};

        // Load model
        let model = Model::new(
            self.model_path
                .as_ref()
                .ok_or_else(|| SttError::NotAvailable("Model path not set".to_string()))?
                .to_str()
                .ok_or_else(|| SttError::NotAvailable("Invalid model path".to_string()))?,
        )
        .ok_or_else(|| SttError::NotAvailable("Failed to load Vosk model".to_string()))?;

        // Create recognizer
        let mut recognizer = Recognizer::new(&model, self.sample_rate)
            .ok_or_else(|| SttError::Transcription("Failed to create recognizer".to_string()))?;

        // Read WAV file
        let mut reader = WavReader::open(audio_path)
            .map_err(|e| SttError::AudioCapture(format!("Failed to open audio file: {}", e)))?;

        let spec = reader.spec();
        if spec.sample_rate != self.sample_rate as u32 {
            return Err(SttError::AudioCapture(format!(
                "Unsupported sample rate: {} (expected {})",
                spec.sample_rate, self.sample_rate
            )));
        }

        // Read audio samples
        let samples: Result<Vec<i16>, _> = reader.samples().collect();
        let samples = samples
            .map_err(|e| SttError::AudioCapture(format!("Failed to read audio samples: {}", e)))?;

        // Process audio - accept_waveform expects &[i16] and returns Result<DecodingState, AcceptWaveformError>
        match recognizer.accept_waveform(&samples) {
            Ok(_) => {
                // Get final result - CompleteResult is an enum with Single/Multiple variants
                let result = recognizer.final_result();
                // Extract text from CompleteResult
                let text = match result {
                    vosk::CompleteResult::Single(single) => single.text,
                    vosk::CompleteResult::Multiple(multiple) => {
                        // Get text from the first (most likely) alternative
                        multiple.alternatives.first().map(|alt| alt.text).unwrap_or("")
                    }
                };

                Ok(text.to_string())
            }
            Err(_) => {
                // On error, try to get partial result - PartialResult has a partial field
                let partial = recognizer.partial_result();
                Ok(partial.partial.to_string())
            }
        }
    }
}

#[cfg(feature = "stt-vosk")]
impl SpeechToTextBackend for VoskBackend {
    fn is_available(&self) -> bool {
        self.model_path.as_ref().map(|p| p.exists()).unwrap_or(false)
    }

    fn transcribe(&self) -> Result<String, SttError> {
        println!("🎤 Recording audio (press Enter to stop)...");
        println!("⚠️  Audio capture from microphone requires additional setup.");
        println!("   For now, please provide the path to a WAV audio file:");
        print!("   Audio file path: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let audio_path = input.trim();

        if audio_path.is_empty() {
            return Err(SttError::AudioCapture("No audio file provided".to_string()));
        }

        let audio_path = PathBuf::from(audio_path);
        if !audio_path.exists() {
            return Err(SttError::AudioCapture(format!(
                "Audio file not found: {}",
                audio_path.display()
            )));
        }

        self.transcribe_audio_file(&audio_path)
    }

    fn name(&self) -> &'static str {
        "vosk"
    }

    fn description(&self) -> &'static str {
        "Vosk offline STT (privacy-focused, no internet required)"
    }
}

/// Speech-to-text manager
pub struct SpeechToTextManager {
    backends: Vec<Box<dyn SpeechToTextBackend>>,
}

impl SpeechToTextManager {
    /// Create a new STT manager with default backends
    pub fn new() -> Self {
        let mut backends: Vec<Box<dyn SpeechToTextBackend>> = Vec::new();

        // Always add text input as fallback (last, so it's only used if nothing else is available)

        // Add cloud API backends if available
        #[cfg(feature = "stt-cloud")]
        {
            if let Ok(openai) = OpenAiWhisperBackend::new(None) {
                backends.push(Box::new(openai));
            }
            if let Ok(google) = GoogleSpeechBackend::new(None) {
                backends.push(Box::new(google));
            }
        }

        // Add vosk offline STT if available
        #[cfg(feature = "stt-vosk")]
        {
            if let Ok(vosk) = VoskBackend::new(None) {
                backends.push(Box::new(vosk));
            }
        }

        // Always add text input as final fallback
        backends.push(Box::new(TextInputBackend));

        Self { backends }
    }

    /// Create a manager with custom backends
    pub fn with_backends(backends: Vec<Box<dyn SpeechToTextBackend>>) -> Self {
        Self { backends }
    }

    /// Get the first available backend
    pub fn get_available_backend(&self) -> Option<&dyn SpeechToTextBackend> {
        self.backends.iter().find(|b| b.is_available()).map(|b| b.as_ref())
    }

    /// Transcribe audio using the first available backend
    pub fn transcribe(&self) -> Result<String, SttError> {
        if let Some(backend) = self.get_available_backend() {
            backend.transcribe()
        } else {
            Err(SttError::NotAvailable("No speech-to-text backend available".to_string()))
        }
    }

    /// List available backends
    pub fn list_backends(&self) -> Vec<&'static str> {
        self.backends.iter().filter(|b| b.is_available()).map(|b| b.name()).collect()
    }
}

impl Default for SpeechToTextManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Interactive voice input with visual feedback
pub struct InteractiveVoiceInput {
    stt_manager: SpeechToTextManager,
}

impl InteractiveVoiceInput {
    pub fn new() -> Self {
        Self {
            stt_manager: SpeechToTextManager::new(),
        }
    }

    /// Prompt user for voice or text input
    pub fn prompt(&self, prompt_text: &str) -> Result<String, SttError> {
        println!("{}", prompt_text);
        println!("💡 Tip: You can type your command or use voice input (if available)");
        println!();

        // For now, always use text input
        // In the future, we can add a prompt asking user to choose
        self.stt_manager.transcribe()
    }

    /// Start continuous listening (for interactive mode)
    ///
    /// Reports available backends and readiness for transcription.
    /// Actual transcription happens via `prompt()` / `transcribe()` calls.
    pub fn start_listening(&self) -> Result<(), SttError> {
        let backends = self.stt_manager.list_backends();
        if backends.len() > 1 || (backends.len() == 1 && backends[0] != "text-input") {
            println!("Voice input ready (backends: {})", backends.join(", "));
        } else {
            println!("Using text input (no audio backend available).");
        }
        Ok(())
    }

    /// Stop listening
    ///
    /// No-op — transcription is one-shot per `prompt()` call, so there
    /// is no persistent session to tear down.
    pub fn stop_listening(&self) -> Result<(), SttError> {
        Ok(())
    }
}

impl Default for InteractiveVoiceInput {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_input_backend() {
        let backend = TextInputBackend;
        assert!(backend.is_available());
        assert_eq!(backend.name(), "text-input");
    }

    #[test]
    fn test_stt_manager_has_text_fallback() {
        let manager = SpeechToTextManager::new();
        assert!(manager.get_available_backend().is_some());
        assert!(manager.list_backends().contains(&"text-input"));
    }
}
