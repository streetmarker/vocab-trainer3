use serde::{Deserialize, Serialize};
use std::fs;
use tauri::{AppHandle, Manager, Runtime};
use base64::{Engine as _, engine::general_purpose};


#[derive(Serialize, Deserialize)]
pub struct TtsRequest {
    pub term: String,
    pub text: String,
}

#[tauri::command]
pub async fn play_or_generate_tts<R: Runtime>(
    app: AppHandle<R>,
    payload: TtsRequest
) -> Result<String, String> {
    // 1. Ustal ścieżkę do katalogu audio w katalogu danych aplikacji
    //    Resource dir (w katalogu instalacyjnym) jest zwykle tylko do odczytu po instalacji.
    let app_data_dir = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;
    let audio_dir = app_data_dir.join("audio");

    // Upewnij się, że katalog istnieje
    if !audio_dir.exists() {
        fs::create_dir_all(&audio_dir).map_err(|e| format!("Failed to create audio dir: {}", e))?;
    }

    // Zweryfikuj i zamień niebezpieczne znaki w nazwie pliku
    let safe_term = payload.term
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect::<String>();
    let file_path = audio_dir.join(format!("{}.mp3", safe_term));

    // 2. Sprawdź czy plik istnieje i ma prawidłowy rozmiar (unika problemów z pustymi/uszkodzonymi plikami)
    if file_path.exists() {
        log::info!("Audio file already exists: {}", file_path.display());
        if let Ok(metadata) = file_path.metadata() {
            let file_size = metadata.len();
            if file_size > 1000 { // MP3 pliki powinny mieć co najmniej ~1KB
                log::info!("Audio file exists and valid: {} ({} bytes)", file_path.display(), file_size);
                // Odczytaj file i zwróć jako data URL (unika sandbox restrictions)
                let audio_bytes = fs::read(&file_path)
                    .map_err(|e| format!("Failed to read audio file: {}", e))?;
                let data_url = format!("data:audio/mp3;base64,{}", general_purpose::STANDARD.encode(&audio_bytes));
                log::info!("Returning cached audio as data URL (size: {}))", file_size);
                return Ok(data_url);
            } else {
                log::warn!("Audio file exists but too small ({} bytes), regenerating...", file_size);
                // Usuń uszkodzony plik
                let _ = fs::remove_file(&file_path);
            }
        }
    }

    // 3. Generuj unikalną nazwę tymczasową żeby uniknąć race conditions
    let temp_file_path = audio_dir.join(format!("{}.tmp.mp3", payload.term));
    log::info!("Will save audio to temp file: {:?}", temp_file_path);


    let client = reqwest::Client::new();
    
    // Get API key from environment (runtime)
    let api_key = std::env::var("API_PROXY_KEY").unwrap_or_default();
    
    if api_key.is_empty() {
        log::warn!("API_PROXY_KEY is empty in environment!");
    } else {
        log::info!("Using API_PROXY_KEY: {}***", &api_key[..5]);
    }

    let response = client
        .post("https://vocab-tts-proxy-1092910876208.europe-west1.run.app/text-to-speech")
        .header("x-api-key", api_key)
        .json(&serde_json::json!({
            "text": payload.text 
        }))
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_body = response.text().await.unwrap_or_default();
        log::error!("Proxy error ({}): {}", status, error_body);
        return Err(format!("Proxy error ({}): {}", status, error_body));
    }

    let json: serde_json::Value = response.json().await.map_err(|e| format!("Failed to parse JSON: {}", e))?;
    let audio_content = json["audioContent"]
        .as_str()
        .ok_or("No audio content in proxy response")?;

    // 4. Dekodowanie Base64 i zapis do TYMCZASOWEGO pliku (unika race conditions)
    let bytes = general_purpose::STANDARD
        .decode(audio_content)
        .map_err(|e| format!("Failed to decode base64: {}", e))?;

    // Zapisz najpierw do pliku tymczasowego
    fs::write(&temp_file_path, &bytes)
        .map_err(|e| format!("Failed to write temp file: {}", e))?;

    // Atomowo przenieś plik tymczasowy na miejsce docelowe
    fs::rename(&temp_file_path, &file_path)
        .map_err(|e| format!("Failed to move temp file to final location: {}", e))?;

    log::info!("Successfully saved audio file: {} ({} bytes)", file_path.display(), bytes.len());

    // Zwróć data URL zamiast file:// (unika Tauri sandbox restrictions)
    let data_url = format!("data:audio/mp3;base64,{}", audio_content);
    log::info!("Returning newly generated audio as data URL");

    Ok(data_url)
}
