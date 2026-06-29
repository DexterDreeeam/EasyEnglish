//! Windows text-to-speech pronunciation playback.

use crate::logging::log_message;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::Duration;

static PLAYBACK_ACTIVE: AtomicBool = AtomicBool::new(false);
const PLAYBACK_TIMEOUT: Duration = Duration::from_secs(10);
const SYNTHESIS_AUDIO_VOLUME: f64 = 1.0;
const SYNTHESIS_SPEAKING_RATE: f64 = 0.72;
const SYNTHESIS_AUDIO_PITCH: f64 = 1.0;
const SSML_PROSODY_VOLUME: &str = "x-loud";

struct PlaybackGuard;

impl Drop for PlaybackGuard {
    fn drop(&mut self) {
        PLAYBACK_ACTIVE.store(false, Ordering::SeqCst);
    }
}

/// Result of requesting pronunciation playback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PronunciationRequest {
    /// Playback was accepted and started on a background worker.
    Started,
    /// The word is not a plain English headword that should be spoken.
    InvalidWord,
    /// A previous pronunciation is still playing, so this request was dropped.
    AlreadyPlaying,
    /// The playback worker could not be created.
    StartFailed,
}

/// Return whether `word` is safe and appropriate for English TTS playback.
pub(crate) fn is_speakable_english_word(word: &str) -> bool {
    let word = word.trim();
    !word.is_empty() && word.chars().all(|ch| ch.is_ascii_alphabetic())
}

/// Attempt to acquire a playback lock.
pub(crate) fn acquire_playback_lock(lock: &AtomicBool) -> bool {
    !lock.swap(true, Ordering::SeqCst)
}

/// Rank an installed voice language for English pronunciation playback.
pub(crate) fn english_voice_rank(language: &str) -> Option<u8> {
    let language = language.to_ascii_lowercase();
    if language == "en-us" {
        Some(0)
    } else if language.starts_with("en-") {
        Some(1)
    } else {
        None
    }
}

/// Rank an installed voice by English locale and gender preference.
pub(crate) fn english_voice_preference_rank(language: &str, is_female: bool) -> Option<u8> {
    let language_rank = english_voice_rank(language)?;
    Some(language_rank * 2 + u8::from(!is_female))
}

/// Request non-blocking Windows TTS playback for an English word.
pub(crate) fn try_speak_english_word(word: &str) -> PronunciationRequest {
    let word = word.trim();
    if !is_speakable_english_word(word) {
        log_message(&format!(
            "[Pronunciation] Ignored invalid English word for playback: '{}'.",
            word
        ));
        return PronunciationRequest::InvalidWord;
    }

    if !acquire_playback_lock(&PLAYBACK_ACTIVE) {
        log_message(&format!(
            "[Pronunciation] Dropped playback request for '{}' because speech is active.",
            word
        ));
        return PronunciationRequest::AlreadyPlaying;
    }

    let word = word.to_string();
    match std::thread::Builder::new()
        .name("easyenglish-pronunciation".to_string())
        .spawn(move || {
            let _guard = PlaybackGuard;
            log_message(&format!(
                "[Pronunciation] Starting Windows TTS for '{}'.",
                word
            ));
            match speak_with_windows_tts(&word) {
                Ok(()) => log_message(&format!(
                    "[Pronunciation] Completed Windows TTS for '{}'.",
                    word
                )),
                Err(err) => log_message(&format!(
                    "[Pronunciation] Windows TTS failed for '{}': {}",
                    word, err
                )),
            }
        }) {
        Ok(_) => PronunciationRequest::Started,
        Err(err) => {
            PLAYBACK_ACTIVE.store(false, Ordering::SeqCst);
            log_message(&format!(
                "[Pronunciation] Failed to start playback worker: {}",
                err
            ));
            PronunciationRequest::StartFailed
        }
    }
}

fn speak_with_windows_tts(word: &str) -> Result<(), String> {
    speak_with_winrt_tts(word)
}

/// Return the in-process Windows pronunciation backend name.
#[cfg(test)]
pub(crate) fn pronunciation_backend_name() -> &'static str {
    "WinRT SpeechSynthesizer"
}

/// Return how long playback may run before the worker treats it as failed.
#[cfg(test)]
pub(crate) fn pronunciation_playback_timeout() -> Duration {
    PLAYBACK_TIMEOUT
}

/// Return the speech tuning used by the WinRT backend.
#[cfg(test)]
pub(crate) fn pronunciation_tuning() -> (f64, f64, f64) {
    (
        SYNTHESIS_AUDIO_VOLUME,
        SYNTHESIS_SPEAKING_RATE,
        SYNTHESIS_AUDIO_PITCH,
    )
}

/// Build SSML used for louder single-word pronunciation.
pub(crate) fn pronunciation_ssml(word: &str, language: &str) -> String {
    format!(
        r#"<speak version="1.0" xml:lang="{language}" xmlns="http://www.w3.org/2001/10/synthesis"><prosody volume="{volume}">{word}</prosody></speak>"#,
        language = escape_xml(language),
        volume = SSML_PROSODY_VOLUME,
        word = escape_xml(word),
    )
}

fn speak_with_winrt_tts(word: &str) -> Result<(), String> {
    use windows::core::{IInspectable, HSTRING};
    use windows::Foundation::TypedEventHandler;
    use windows::Media::Core::MediaSource;
    use windows::Media::Playback::{
        MediaPlayer, MediaPlayerAudioCategory, MediaPlayerFailedEventArgs,
    };
    use windows::Media::SpeechSynthesis::SpeechSynthesizer;
    use windows::Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED};

    struct WinrtApartment;

    impl WinrtApartment {
        fn initialize() -> Result<Self, String> {
            unsafe {
                RoInitialize(RO_INIT_MULTITHREADED)
                    .map_err(|err| format_winrt_error("initialize WinRT", err))?;
            }
            Ok(Self)
        }
    }

    impl Drop for WinrtApartment {
        fn drop(&mut self) {
            unsafe {
                RoUninitialize();
            }
        }
    }

    let _apartment = WinrtApartment::initialize()?;
    let synthesizer = SpeechSynthesizer::new()
        .map_err(|err| format_winrt_error("create SpeechSynthesizer", err))?;
    let voice_language = configure_english_voice(&synthesizer)?;
    configure_speech_options(&synthesizer)?;
    let ssml = pronunciation_ssml(word, voice_language.as_deref().unwrap_or("en-US"));
    let stream = synthesizer
        .SynthesizeSsmlToStreamAsync(&HSTRING::from(ssml))
        .map_err(|err| format_winrt_error("start speech synthesis", err))?
        .get()
        .map_err(|err| format_winrt_error("finish speech synthesis", err))?;
    let content_type = stream
        .ContentType()
        .map_err(|err| format_winrt_error("read speech stream content type", err))?;
    let source = MediaSource::CreateFromStream(&stream, &content_type)
        .map_err(|err| format_winrt_error("create media source", err))?;
    let player = MediaPlayer::new().map_err(|err| format_winrt_error("create MediaPlayer", err))?;
    player
        .SetAutoPlay(false)
        .map_err(|err| format_winrt_error("disable MediaPlayer autoplay", err))?;
    player
        .SetVolume(1.0)
        .map_err(|err| format_winrt_error("set MediaPlayer volume", err))?;
    player
        .SetAudioCategory(MediaPlayerAudioCategory::Speech)
        .map_err(|err| format_winrt_error("set MediaPlayer audio category", err))?;

    let (tx, rx) = mpsc::channel::<Result<(), String>>();
    let ended_tx = tx.clone();
    let ended_token = player
        .MediaEnded(&TypedEventHandler::<MediaPlayer, IInspectable>::new(
            move |_, _| {
                let _ = ended_tx.send(Ok(()));
                Ok(())
            },
        ))
        .map_err(|err| format_winrt_error("register MediaEnded handler", err))?;

    let failed_tx = tx;
    let failed_token = player
        .MediaFailed(
            &TypedEventHandler::<MediaPlayer, MediaPlayerFailedEventArgs>::new(move |_, args| {
                let message = args
                    .as_ref()
                    .and_then(|value| value.ErrorMessage().ok())
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "MediaPlayer failed without an error message".to_string());
                let code = args
                    .as_ref()
                    .and_then(|value| value.ExtendedErrorCode().ok())
                    .map(|value| format!("0x{:08x}", value.0 as u32))
                    .unwrap_or_else(|| "unavailable error code".to_string());
                let _ = failed_tx.send(Err(format!("{message} ({code})")));
                Ok(())
            }),
        )
        .map_err(|err| format_winrt_error("register MediaFailed handler", err))?;

    player
        .SetSource(&source)
        .map_err(|err| format_winrt_error("set MediaPlayer source", err))?;
    player
        .Play()
        .map_err(|err| format_winrt_error("start MediaPlayer playback", err))?;

    let playback_result = rx
        .recv_timeout(PLAYBACK_TIMEOUT)
        .unwrap_or_else(|_| Err("WinRT MediaPlayer playback timed out".to_string()));

    let _ = player.RemoveMediaEnded(ended_token);
    let _ = player.RemoveMediaFailed(failed_token);
    let _ = player.Close();
    let _ = stream.Close();
    let _ = synthesizer.Close();

    playback_result
}

fn configure_english_voice(
    synthesizer: &windows::Media::SpeechSynthesis::SpeechSynthesizer,
) -> Result<Option<String>, String> {
    use windows::Media::SpeechSynthesis::{SpeechSynthesizer, VoiceGender, VoiceInformation};

    let voices = match SpeechSynthesizer::AllVoices() {
        Ok(voices) => voices,
        Err(err) => {
            log_message(&format!(
                "[Pronunciation] Could not enumerate WinRT voices; using default voice: {}",
                format_winrt_error("enumerate voices", err)
            ));
            return Ok(None);
        }
    };

    let voice_count = voices
        .Size()
        .map_err(|err| format_winrt_error("read installed voice count", err))?;
    let mut selected: Option<(VoiceInformation, String, String, bool, u8)> = None;

    for index in 0..voice_count {
        let voice = voices
            .GetAt(index)
            .map_err(|err| format_winrt_error("read installed voice", err))?;
        let language = voice
            .Language()
            .map_err(|err| format_winrt_error("read installed voice language", err))?
            .to_string();
        let display_name = voice
            .DisplayName()
            .map(|value| value.to_string())
            .unwrap_or_else(|_| "<unknown>".to_string());
        let is_female = voice
            .Gender()
            .map(|gender| gender == VoiceGender::Female)
            .unwrap_or(false);
        let Some(rank) = english_voice_preference_rank(&language, is_female) else {
            continue;
        };
        if selected
            .as_ref()
            .map(|(_, _, _, _, selected_rank)| rank < *selected_rank)
            .unwrap_or(true)
        {
            selected = Some((voice, language, display_name, is_female, rank));
        }
        if rank == 0 {
            break;
        }
    }

    if let Some((voice, language, display_name, is_female, _)) = selected {
        synthesizer
            .SetVoice(&voice)
            .map_err(|err| format_winrt_error("set English voice", err))?;
        log_message(&format!(
            "[Pronunciation] Using WinRT voice '{}' language '{}' gender '{}'.",
            display_name,
            language,
            if is_female { "female" } else { "non-female" }
        ));
        Ok(Some(language))
    } else {
        log_message("[Pronunciation] No English WinRT voice found; using default voice.");
        Ok(None)
    }
}

fn configure_speech_options(
    synthesizer: &windows::Media::SpeechSynthesis::SpeechSynthesizer,
) -> Result<(), String> {
    let options = synthesizer
        .Options()
        .map_err(|err| format_winrt_error("read speech options", err))?;
    options
        .SetAudioVolume(SYNTHESIS_AUDIO_VOLUME)
        .map_err(|err| format_winrt_error("set speech volume", err))?;
    options
        .SetSpeakingRate(SYNTHESIS_SPEAKING_RATE)
        .map_err(|err| format_winrt_error("set speech rate", err))?;
    options
        .SetAudioPitch(SYNTHESIS_AUDIO_PITCH)
        .map_err(|err| format_winrt_error("set speech pitch", err))?;
    log_message(&format!(
        "[Pronunciation] WinRT tuning volume={SYNTHESIS_AUDIO_VOLUME:.2} rate={SYNTHESIS_SPEAKING_RATE:.2} pitch={SYNTHESIS_AUDIO_PITCH:.2} prosody_volume={SSML_PROSODY_VOLUME}."
    ));
    Ok(())
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn format_winrt_error(context: &str, err: windows::core::Error) -> String {
    format!("{context}: {err} (HRESULT=0x{:08x})", err.code().0 as u32)
}
