use std::{error::Error, fs, env};
use serde_json::Value;
use tokio::{io::AsyncReadExt, net::TcpListener};
use url::form_urlencoded;
use serde::{Deserialize, Serialize};
use open;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Read configuration from file
    let mut token_response = TokenResponse::from_file("config.json")?;

    if token_response.client_id.is_empty() || token_response.client_secret.is_empty() {
        eprintln!("Missing info: client_id or client_secret_id")
    } else {
        // Call update_twitch_authentication using values from configuration
        match update_twitch_authentication(token_response).await {
            Ok(updated_token_response) => {
                // Update the local token_response with the updated one
                token_response = updated_token_response;

                // Write updated configuration back to file
                token_response.to_file("config.json")?;
            }
            Err(e) => {
                println!("Error updating Twitch authentication: {}", e);
            }
        }
    }

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct TokenResponse {
    client_id: String,
    client_secret: String,
    scopes: Vec<String>,
    refresh_token: String,
    oauth_token: String,
}

impl TokenResponse {
    // Read configuration from file
    fn from_file(filename: &str) -> Result<Self, Box<dyn Error>> {
        // Get the current directory where the executable is located
        let current_dir = env::current_dir()?;
        let config_path = current_dir.join(filename);

        match fs::read_to_string(&config_path) {
            Ok(contents) => {
                let config: TokenResponse = serde_json::from_str(&contents)?;
                Ok(config)
            }
            Err(err) => {
                if err.kind() == std::io::ErrorKind::NotFound {
                    // File doesn't exist, create a default configuration and write it to the file
                    let default_config = TokenResponse {
                        client_id: String::new(),
                        client_secret: String::new(),
                        scopes: Vec::new(),
                        refresh_token: String::new(),
                        oauth_token: String::new(),
                    };
                    default_config.to_file(&config_path.into_os_string().into_string().unwrap())?;
                    Ok(default_config)
                } else {
                    Err(Box::new(err))
                }
            }
        }
    }

    // Write configuration to file
    fn to_file(&self, filename: &str) -> Result<(), Box<dyn Error>> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(filename, json)?;
        Ok(())
    }
}

async fn update_twitch_authentication(mut info: TokenResponse) -> Result<TokenResponse, Box<dyn Error>> {
    let twitch_refresh = &info.refresh_token; // Your refresh token here
    let scopes = &info.scopes; // Scopes you're requesting
    let client_id = &info.client_id;
    let client_secret = &info.client_secret;

    if twitch_refresh.is_empty() {
        let scopes_str = scopes.join("+");
        let listener = TcpListener::bind("localhost:8080").await?;
        let auth_url = format!("https://id.twitch.tv/oauth2/authorize?client_id={}&redirect_uri=http://localhost:8080/&response_type=code&scope={}", client_id, scopes_str);

        println!("Opening auth_url");
        if let Err(err) = open::that(auth_url) {
            println!("Error opening URL: {}", err);
        }

        let (mut stream, _) = listener.accept().await?;

        let mut buffer = [0; 1024];
        let n = stream.read(&mut buffer).await?;

        if n == 0 {
            return Err("Failed reading stream buffer.".into());
        }

        let request = std::str::from_utf8(&buffer[..n])?;

        // Find the index of "code="
        let s = request.find("code=").unwrap_or(0) + "code=".len();
    
        // Find the index of "&" after "code="
        let e = request[s..].find('&').unwrap_or(request.len());
    
        // Extract the code substring
        let code = &request[s..s + e];

        if code.is_empty() {
            return Err("Failed to get code.".into());
        }

        let token_url = "https://id.twitch.tv/oauth2/token";

        let params = form_urlencoded::Serializer::new(String::new())
            .append_pair("client_id", &client_id)
            .append_pair("client_secret", &client_secret)
            .append_pair("code", &code)
            .append_pair("grant_type", "authorization_code")
            .append_pair("redirect_uri", "http://localhost:8080/")
            .finish();

        let client = reqwest::Client::new();

        let response = client
            .post(token_url)
            .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(params)
            .send()
            .await?;

        match response.status().as_u16() {
            200..=299 => {
                let body = response.text().await?;
                let resp: Result<Value, _> = serde_json::from_str(&body);

                match resp {
                    Ok(json) => {
                        println!("Authentication successful.");
                        info.oauth_token = format!("{}", json["access_token"].as_str().unwrap_or(""));
                        info.refresh_token = format!("{}", json["refresh_token"].as_str().unwrap_or(""));
                        println!("Access token: {}", info.oauth_token);
                        println!("Refresh token: {}", info.refresh_token);
                    }
                    Err(_e) => {
                        return Err("Failed parsing json response.")?;
                    }
                }
            }
            400..=599 => {
                let status = response.status();
                let error_message = response.text().await?;
                return Err(format!("Error {}: {}", status, error_message))?;
            }
            _ => {
                return Err(format!("Unexpected status code: {}", response.status()))?;
            }
        }
    } else {
        let token_url = "https://id.twitch.tv/oauth2/token";

        let params = form_urlencoded::Serializer::new(String::new())
            .append_pair("client_id", &client_id)
            .append_pair("client_secret", &client_secret)
            .append_pair("grant_type", "refresh_token")
            .append_pair("refresh_token", &twitch_refresh)
            .finish();

        let client = reqwest::Client::new();
        let response = client
            .post(token_url)
            .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(params)
            .send()
            .await?;

        match response.status().as_u16() {
            200..=299 => {
                let body = response.text().await?;
                let resp: Result<Value, _> = serde_json::from_str(&body);

                match resp {
                    Ok(json) => {
                        println!("Authentication successful.");
                        info.oauth_token = format!("{}", json["access_token"].as_str().unwrap_or(""));
                        info.refresh_token = format!("{}", json["refresh_token"].as_str().unwrap_or(""));
                        println!("Access token: {}", info.oauth_token);
                        println!("Refresh token: {}", info.refresh_token);
                    }
                    Err(_e) => {
                        return Err("Failed parsing json response.")?;
                    }
                }
            }
            400..=599 => {
                let status = response.status();
                let error_message = response.text().await?;
                return Err(format!("Error {}: {}", status, error_message))?;
            }
            _ => {
                return Err(format!("Unexpected status code: {}", response.status()))?;
            }           
        }
    }
    Ok(info)
}