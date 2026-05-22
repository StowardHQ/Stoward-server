use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderValue};
use rusqlite::params;
use serde::Deserialize;
use std::env;
use std::time::Duration;
use tokio::time::interval;

use crate::db::DbState;

#[derive(Deserialize)]
struct StoatIcon {
    _id: String,
}

#[derive(Deserialize)]
struct StoatBanner {
    _id: String,
}

#[derive(Deserialize)]
struct StoatServerResponse {
    #[serde(rename = "name")]
    server_name: String,
    description: Option<String>,
    #[serde(rename = "icon")]
    server_icon: Option<StoatIcon>,
    #[serde(rename = "banner")]
    server_banner: Option<StoatBanner>,
    #[serde(rename = "owner")]
    owner_id: Option<String>,
}

#[derive(Deserialize)]
struct StoatInviteResponse {
    user_name: Option<String>,
    member_count: Option<i64>,
}

pub async fn start_refresh_job(db: DbState) {
    let mut sched = interval(Duration::from_secs(4 * 60 * 60)); // 4 hours

    let bot_token =
        env::var("BOT_TOKEN").expect("BOT_TOKEN environment variable must be set in .env");

    let mut headers = HeaderMap::new();
    let mut auth_value =
        HeaderValue::from_str(&bot_token).expect("Invalid characters in BOT_TOKEN");
    auth_value.set_sensitive(true);

    headers.insert(
        reqwest::header::HeaderName::from_static("x-bot-token"),
        auth_value,
    );

    let client = Client::builder()
        .default_headers(headers)
        .build()
        .expect("Failed to build authenticated reqwest client");

    loop {
        sched.tick().await;
        println!("[{}] Refreshing...", chrono::Utc::now().to_rfc3339());

        let servers_to_sync: Vec<(String, Option<String>)> = {
            let conn = db.lock().unwrap();
            let mut stmt = conn
                .prepare("SELECT server_id, invite_link FROM servers")
                .unwrap();
            let rows = stmt
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
                })
                .unwrap();
            rows.filter_map(Result::ok).collect()
        };

        for (server_id, invite_link) in servers_to_sync {
            let server_url = format!("https://stoat.chat/api/servers/{}", server_id);

            let mut synced_name = None;
            let mut synced_description = None;
            let mut synced_icon = None;
            let mut synced_banner = None;
            let mut synced_owner_id = None;

            match client.get(&server_url).send().await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        if let Ok(data) = resp.json::<StoatServerResponse>().await {
                            synced_name = Some(data.server_name);
                            synced_description = data.description;
                            synced_icon = data.server_icon.map(|i| {
                                format!(
                                    "https://cdn.stoatusercontent.com/icons/{}?max_side=256",
                                    i._id
                                )
                            });
                            synced_banner = data.server_banner.map(|b| {
                                format!("https://cdn.stoatusercontent.com/banners/{}", b._id)
                            });
                            synced_owner_id = data.owner_id;
                        } else {
                            eprintln!("Failed to parse server JSON response for {}", server_id);
                        }
                    } else {
                        eprintln!(
                            "API error response for server {}: status {}",
                            server_id,
                            resp.status()
                        );
                    }
                }
                Err(e) => eprintln!("Could not reach server API for {}: {}", server_id, e),
            }

            let mut synced_user_name = None;
            let mut synced_member_count = None;

            if let Some(code) = invite_link
                .as_ref()
                .and_then(|link| link.split('/').next_back())
            {
                let invite_url = format!("https://stoat.chat/api/invites/{}", code);

                match client.get(&invite_url).send().await {
                    Ok(resp) => {
                        if resp.status().is_success() {
                            if let Ok(invite_data) = resp.json::<StoatInviteResponse>().await {
                                synced_user_name = invite_data.user_name;
                                synced_member_count = invite_data.member_count;
                            } else {
                                eprintln!("Failed to parse invite JSON response for code {}", code);
                            }
                        } else {
                            eprintln!(
                                "API error response for invite code {}: status {}",
                                code,
                                resp.status()
                            );
                        }
                    }
                    Err(e) => eprintln!("Could not reach invite API for code {}: {}", code, e),
                }
            }

            let conn = db.lock().unwrap();
            let _ = conn.execute(
                "UPDATE servers SET 
                    server_name = COALESCE(?, server_name),
                    description = COALESCE(?, description),
                    icon_url = COALESCE(?, icon_url),
                    banner_url = COALESCE(?, banner_url),
                    owner_id = COALESCE(?, owner_id),
                    owner = COALESCE(?, owner),
                    members = COALESCE(?, members)
                 WHERE server_id = ?",
                params![
                    synced_name,
                    synced_description,
                    synced_icon,
                    synced_banner,
                    synced_owner_id,
                    synced_user_name,
                    synced_member_count,
                    server_id
                ],
            );
        }
        println!("Refresh complete.");
    }
}
