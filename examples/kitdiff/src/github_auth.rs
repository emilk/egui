use eframe::egui;
use ehttp;
use serde_json;
use std::fmt;
use std::sync::mpsc;
#[cfg(not(target_arch = "wasm32"))]
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct AuthState {
    logged_in: Option<LoggedInState>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LoggedInState {
    access_token: String,
    provider_token: Option<String>, // GitHub OAuth token
    expires_at: u64,
    username: String,
}


#[derive(Debug)]
pub struct GitHubAuth {
    supabase_url: String,
    supabase_anon_key: String,
    state: AuthState,
    auth_sender: AuthSender,
    auth_receiver: AuthReceiver,
}

#[derive(Debug)]
pub enum AuthError {
    NetworkError(String),
    ParseError(String),
    AuthenticationError(String),
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            AuthError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            AuthError::AuthenticationError(msg) => write!(f, "Authentication error: {}", msg),
        }
    }
}

impl std::error::Error for AuthError {}

#[derive(Debug, Clone)]
pub enum AuthEvent {
    LoginSuccessful(AuthState),
    LogoutCompleted,
    Error(String),
}

pub type AuthSender = mpsc::Sender<AuthEvent>;
pub type AuthReceiver = mpsc::Receiver<AuthEvent>;

// Helper function to get current timestamp in seconds
fn get_current_timestamp() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        // Use JavaScript Date.now() for WASM
        (js_sys::Date::now() / 1000.0) as u64
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
}

// URL parsing utilities
pub fn parse_github_artifact_url(url: &str) -> Option<(String, String, String)> {
    // Expected format: github.com/owner/repo/actions/runs/12345/artifacts/67890
    let url = url
        .trim_start_matches("https://")
        .trim_start_matches("http://");

    let parts: Vec<&str> = url.split('/').collect();
    if parts.len() >= 7
        && parts[0] == "github.com"
        && parts[3] == "actions"
        && parts[4] == "runs"
        && parts[6] == "artifacts"
        && parts.len() >= 8
    {
        Some((
            parts[1].to_string(), // owner
            parts[2].to_string(), // repo
            parts[7].to_string(), // artifact_id
        ))
    } else {
        None
    }
}

pub fn github_artifact_api_url(owner: &str, repo: &str, artifact_id: &str) -> String {
    format!(
        "https://api.github.com/repos/{}/{}/actions/artifacts/{}/zip",
        owner, repo, artifact_id
    )
}

impl GitHubAuth {
    pub fn new(state: AuthState) -> Self {
        // Supabase configuration
        let supabase_url = "https://fqhsaeyjqrjmlkqflvho.supabase.co".to_string(); // Replace with your project
        let supabase_anon_key = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6ImZxaHNhZXlqcXJqbWxrcWZsdmhvIiwicm9sZSI6ImFub24iLCJpYXQiOjE3NTgyMTk4MzIsImV4cCI6MjA3Mzc5NTgzMn0.TuhMjHhBCNyKquyVWq3djOfpBVDhcpSmNRWSErpseuw".to_string(); // Replace with your anon key

        let (auth_sender, auth_receiver) = mpsc::channel();

        Self {
            supabase_url,
            supabase_anon_key,
            state,
            auth_sender,
            auth_receiver,
        }
    }

    pub fn login_github(&self) {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(window) = web_sys::window() {
                if let Ok(origin) = window.location().origin() {
                    let auth_url = format!(
                        "{}/auth/v1/authorize?provider=github&redirect_to={}&scopes=repo",
                        self.supabase_url, origin
                    );

                    let _ = window.location().set_href(&auth_url);
                }
            }
        }
    }


    pub fn check_for_auth_callback(&mut self) {
        #[cfg(target_arch = "wasm32")]
        {
            // Check if we have auth tokens in the URL fragment
            if let Some(window) = web_sys::window() {
                if let Ok(hash) = window.location().hash() {
                    if hash.contains("access_token") {
                        // Parse tokens directly from URL fragment
                        let tokens = self.parse_url_fragment(&hash);

                        if let (Some(access_token), Some(provider_token)) = (tokens.get("access_token"), tokens.get("provider_token")) {
                            let sender = self.auth_sender.clone();
                            let github_token = provider_token.clone();

                            let access_token = access_token.clone();

                            wasm_bindgen_futures::spawn_local(async move {
                                match Self::fetch_user_info(&github_token).await {
                                    Ok(username) => {
                                        let expires_at = get_current_timestamp() + (24 * 60 * 60); // 24 hours

                                        let logged_in_state = LoggedInState {
                                            access_token: access_token.clone(),
                                            provider_token: Some(github_token),
                                            expires_at,
                                            username,
                                        };
                                        let auth_state = AuthState {
                                            logged_in: Some(logged_in_state),
                                        };
                                        let _ = sender.send(AuthEvent::LoginSuccessful(auth_state));

                                        // Clear the URL hash
                                        if let Some(window) = web_sys::window() {
                                            if let Ok(history) = window.history() {
                                                let _ = history.replace_state_with_url(
                                                    &wasm_bindgen::JsValue::NULL,
                                                    "",
                                                    Some("./"),
                                                );
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        let _ = sender.send(AuthEvent::Error(format!(
                                            "Failed to fetch user info: {}",
                                            e
                                        )));
                                    }
                                }
                            });
                        } else {
                            let _ = self.auth_sender.send(AuthEvent::Error(
                                "Missing required tokens in OAuth callback".to_string(),
                            ));
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn parse_url_fragment(&self, hash: &str) -> std::collections::HashMap<String, String> {
        let mut params = std::collections::HashMap::new();

        // Remove the leading # if present
        let hash = hash.strip_prefix('#').unwrap_or(hash);

        // Parse key=value pairs separated by &
        for pair in hash.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                // URL decode the value
                let decoded_value = js_sys::decode_uri_component(value)
                    .unwrap_or_else(|_| value.into())
                    .as_string()
                    .unwrap_or_else(|| value.to_string());
                params.insert(key.to_string(), decoded_value);
            }
        }

        params
    }

    async fn fetch_user_info(token: &str) -> Result<String, AuthError> {
        let mut request = ehttp::Request::get("https://api.github.com/user");
        request
            .headers
            .insert("Authorization".to_string(), format!("Bearer {}", token));
        request
            .headers
            .insert("User-Agent".to_string(), "kitdiff-app".to_string());

        let response = ehttp::fetch_async(request)
            .await
            .map_err(|e| AuthError::NetworkError(format!("Failed to fetch user info: {}", e)))?;

        if response.status == 200 {
            let user_info: serde_json::Value = serde_json::from_slice(&response.bytes)
                .map_err(|e| AuthError::ParseError(format!("Failed to parse user info: {}", e)))?;

            let username = user_info["login"]
                .as_str()
                .ok_or_else(|| AuthError::ParseError("Username not found in response".to_string()))?
                .to_string();

            Ok(username)
        } else {
            Err(AuthError::NetworkError(format!(
                "GitHub API returned status: {}",
                response.status
            )))
        }
    }

    pub fn is_authenticated(&self) -> bool {
        if let Some(state) = &self.state.logged_in {
            let now = get_current_timestamp();
            return now < state.expires_at;
        }
        false
    }

    pub fn get_username(&self) -> Option<&str> {
        self.state.logged_in.as_ref().map(|s| s.username.as_str())
    }

    pub fn get_token(&self) -> Option<&str> {
        if self.is_authenticated() {
            self.state
                .logged_in
                .as_ref()
                .and_then(|s| s.provider_token.as_deref())
        } else {
            None
        }
    }

    pub fn logout(&mut self) {
        self.state.logged_in = None;
        let _ = self.auth_sender.send(AuthEvent::LogoutCompleted);
    }

    pub fn get_auth_state(&self) -> &AuthState {
        &self.state
    }

    pub fn update(&mut self, _ctx: &egui::Context) {
        // Check for auth callback in URL
        self.check_for_auth_callback();

        // Check for messages from auth flow
        while let Ok(event) = self.auth_receiver.try_recv() {
            match event {
                AuthEvent::LoginSuccessful(state) => {
                    self.state = state;
                }
                AuthEvent::Error(error) => {
                    eprintln!("Auth error: {}", error);
                }
                _ => {}
            }
        }
    }
}
