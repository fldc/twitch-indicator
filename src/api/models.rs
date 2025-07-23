#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct User {
    pub id: String,
    pub login: String,
    pub display_name: String,
    pub profile_image_url: String,
    pub view_count: u64,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Stream {
    pub id: String,
    pub user_id: String,
    pub user_login: String,
    pub user_name: String,
    pub game_id: String,
    pub game_name: String,
    pub title: String,
    pub viewer_count: u32,
    pub started_at: String,
    pub language: String,
    pub thumbnail_url: String,
    pub tag_ids: Vec<String>,
    pub is_mature: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FollowedChannel {
    pub broadcaster_id: String,
    pub broadcaster_login: String,
    pub broadcaster_name: String,
    pub followed_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Game {
    pub id: String,
    pub name: String,
    pub box_art_url: String,
}

#[derive(Debug, Deserialize)]
pub struct TwitchResponse<T> {
    pub data: Vec<T>,
    pub pagination: Option<Pagination>,
}

#[derive(Debug, Deserialize)]
pub struct Pagination {
    pub cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TokenValidation {
    pub client_id: String,
    pub login: String,
    pub scopes: Vec<String>,
    pub user_id: String,
    pub expires_in: u64,
}

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_type: String,
    pub scope: Vec<String>,
}

impl Stream {
    pub fn formatted_viewer_count(&self) -> String {
        format_viewer_count(self.viewer_count)
    }

    pub fn url(&self) -> String {
        format!("https://www.twitch.tv/{}", self.user_login)
    }

    pub fn thumbnail_with_size(&self, width: u32, height: u32) -> String {
        self.thumbnail_url
            .replace("{width}", &width.to_string())
            .replace("{height}", &height.to_string())
    }
}

impl User {
    pub fn profile_image_with_size(&self, size: u32) -> String {
        self.profile_image_url
            .replace("300x300", &format!("{size}x{size}"))
    }
}

pub fn format_viewer_count(count: u32) -> String {
    if count >= 1_000_000 {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    } else if count >= 10_000 {
        format!("{}K", count / 1_000)
    } else if count >= 1_000 {
        format!("{:.1}K", count as f64 / 1_000.0)
    } else {
        count.to_string()
    }
}
