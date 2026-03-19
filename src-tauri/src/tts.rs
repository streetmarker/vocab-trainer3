use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager, Runtime};
use base64::{Engine as _, engine::general_purpose};
use jsonwebtoken::{encode, EncodingKey, Header};

// Struktura dla JWT payload
#[derive(Serialize)]
struct JwtClaims {
    iss: String,
    scope: String,
    aud: String,
    exp: u64,
    iat: u64,
}

// Struktura do parsowania Google Service Account JSON
#[derive(Deserialize)]
struct GoogleServiceAccount {
    #[serde(default)]
    type_: String,
    #[serde(default)]
    project_id: String,
    #[serde(default)]
    private_key_id: String,
    #[serde(default)]
    private_key: String,
    #[serde(default)]
    client_email: String,
    #[serde(default)]
    client_id: String,
    #[serde(default)]
    auth_uri: String,
    #[serde(default)]
    token_uri: String,
}

// Struktura dla Google OAuth token response
#[derive(Deserialize)]
struct GoogleTokenResponse {
    access_token: String,
    #[serde(default)]
    expires_in: u64,
}

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
    // 1. Ustal ścieżkę do katalogu audio (zawsze public/assets/audio dla asset:// dostępu)
    let resource_dir = app.path().resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {}", e))?;
    let audio_dir = resource_dir.join("../public/assets/audio");

    // Upewnij się, że katalog istnieje
    if !audio_dir.exists() {
        fs::create_dir_all(&audio_dir).map_err(|e| format!("Failed to create audio dir: {}", e))?;
    }

    let file_path = audio_dir.join(format!("{}.mp3", payload.term));

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

    // 3. Czytaj GOOGLE_APPLICATION_CREDENTIALS (standardowy sposób Google Cloud)
    let creds_path = std::env::var("GOOGLE_APPLICATION_CREDENTIALS")
        .map_err(|_| "Missing GOOGLE_APPLICATION_CREDENTIALS env var".to_string())?;
    
    // Odczytaj i sparsuj plik JSON
    let creds_file = fs::read_to_string(&creds_path)
        .map_err(|e| format!("Failed to read credentials file: {}", e))?;
    
    let creds: GoogleServiceAccount = serde_json::from_str(&creds_file)
        .map_err(|e| format!("Failed to parse credentials JSON: {}", e))?;

    // 3a. Wygeneruj JWT token
    let jwt = generate_jwt(&creds)?;
    log::info!("Generated JWT token for Service Account: {}", creds.client_email);

    // 3b. Wymień JWT na access_token
    let client = reqwest::Client::new();
    let token_response = client
        .post(&creds.token_uri)
        .form(&[
            ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
            ("assertion", &jwt),
        ])
        .send()
        .await
        .map_err(|e| format!("Failed to get access token: {}", e))?;

    let token_data: GoogleTokenResponse = token_response
        .json()
        .await
        .map_err(|e| format!("Failed to parse token response: {}", e))?;
    
    let access_token = token_data.access_token;
    log::info!("Obtained access_token from Google OAuth, expires in {} seconds", token_data.expires_in);

    // 3c. Wyślij request do Google TTS API z access_token w Authorization headera
    log::info!("Sending TTS request for term: {}", payload.term);
    log::info!("TTS request payload: {}", payload.text);
    
    let response = client
        .post("https://texttospeech.googleapis.com/v1/text:synthesize")
        .header("Authorization", format!("Bearer {}", access_token))
        .json(&serde_json::json!({
            "input": { "text": payload.text },
            "voice": { "languageCode": "en-US", "ssmlGender": "NEUTRAL" },
            "audioConfig": { "audioEncoding": "MP3" }
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let json: serde_json::Value = response.json().await.map_err(|e| e.to_string())?;
    // log::info!("TTS response JSON: {}", json);
    let audio_content = json["audioContent"]
        .as_str()
        .ok_or("No audio content in response")?;

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

// Funkcja do generowania JWT z Service Account
fn generate_jwt(service_account: &GoogleServiceAccount) -> Result<String, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();

    let claims = JwtClaims {
        iss: service_account.client_email.clone(),
        scope: "https://www.googleapis.com/auth/cloud-platform".to_string(),
        aud: "https://oauth2.googleapis.com/token".to_string(),
        exp: now + 3600,
        iat: now,
    };

    let encoding_key = EncodingKey::from_rsa_pem(service_account.private_key.as_bytes())
        .map_err(|e| format!("Failed to parse private key: {}", e))?;

    encode(&Header::new(jsonwebtoken::Algorithm::RS256), &claims, &encoding_key)
        .map_err(|e| e.to_string())
}
