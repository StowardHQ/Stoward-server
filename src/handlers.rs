use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

use crate::db::DbState;

#[derive(Serialize, Deserialize, Clone)]
pub struct Server {
    pub id: Option<i64>,
    pub server_id: String,
    pub server_name: String,
    pub icon_url: Option<String>,
    pub banner_url: Option<String>,
    pub invite_link: Option<String>,
    pub members: i64,
    pub description: Option<String>,
    pub is_verified: i64,
    pub is_nsfw: i64,
    pub owner: Option<String>,
    pub owner_id: Option<String>,
    pub added_on: Option<String>,
    pub last_bumped: Option<String>,
}

#[derive(Deserialize)]
pub struct SyncServerInput {
    pub server_id: String,
    pub server_name: String,
    pub icon_url: Option<String>,
    pub banner_url: Option<String>,
    pub invite_link: String,
    pub members: Option<i64>,
    pub description: Option<String>,
    pub is_nsfw: Option<i64>,
    pub owner: Option<String>,
    pub owner_id: Option<String>,
}

#[derive(Deserialize)]
pub struct PatchServerInput {
    pub is_verified: Option<i64>,
    pub is_nsfw: Option<i64>,
    pub description: Option<String>,
}

// Check if listing is banned
fn check_if_banned(conn: &Connection, server_id: &str) -> Result<(), (StatusCode, String)> {
    let is_banned: Option<i64> = conn
        .query_row(
            "SELECT is_banned FROM servers WHERE server_id = ?",
            [server_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if let Some(1) = is_banned {
        return Err((
            StatusCode::FORBIDDEN,
            "This listing is banned from discovery.".to_string(),
        ));
    }
    Ok(())
}

// For API tokens for certain API calls.
fn authorize_request(headers: &HeaderMap) -> Result<(), (StatusCode, String)> {
    let expected_key = env::var("DISCOVERY_API_KEY").map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Server configuration error: DISCOVERY_API_KEY not set.".to_string(),
        )
    })?;

    let api_key = headers
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    if api_key.is_empty() || api_key != expected_key {
        return Err((StatusCode::UNAUTHORIZED, "Unauthorized".to_string()));
    }

    Ok(())
}
// GET /api/servers
pub async fn get_servers(
    State(db): State<DbState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Vec<Server>>, (StatusCode, String)> {
    let q = params.get("q");
    let sort = params.get("sort").map(|s| s.as_str()).unwrap_or("bumps");

    let mut query_str = String::from(
        "SELECT id, server_id, server_name, icon_url, banner_url, invite_link, members, description, is_verified, is_nsfw, owner, owner_id, added_on, last_bumped FROM servers WHERE is_banned = 0",
    );
    let mut sql_params: Vec<String> = Vec::new();

    if let Some(search) = q {
        query_str.push_str(" AND (server_name LIKE ? OR description LIKE ?)");
        let search_pattern = format!("%{}%", search);
        sql_params.push(search_pattern.clone());
        sql_params.push(search_pattern);
    }

    match sort {
        "members" => query_str.push_str(" ORDER BY members DESC"),
        "activity" => {
            query_str.push_str(" ORDER BY is_verified DESC, members DESC, last_bumped DESC")
        }
        "newest" => query_str.push_str(" ORDER BY added_on DESC"),
        _ => query_str.push_str(" ORDER BY last_bumped DESC"),
    }

    let conn = db.lock().unwrap();
    let mut stmt = conn
        .prepare(&query_str)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let param_refs: Vec<&dyn rusqlite::ToSql> = sql_params
        .iter()
        .map(|s| s as &dyn rusqlite::ToSql)
        .collect();

    let server_rows = stmt
        .query_map(&param_refs[..], |row| {
            Ok(Server {
                id: row.get(0)?,
                server_id: row.get(1)?,
                server_name: row.get(2)?,
                icon_url: row.get(3)?,
                banner_url: row.get(4)?,
                invite_link: row.get(5)?,
                members: row.get(6)?,
                description: row.get(7)?,
                is_verified: row.get(8)?,
                is_nsfw: row.get(9)?,
                owner: row.get(10)?,
                owner_id: row.get(11)?,
                added_on: row.get(12)?,
                last_bumped: row.get(13)?,
            })
        })
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut servers = Vec::new();
    for server in server_rows {
        servers.push(server.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?);
    }

    Ok(Json(servers))
}

// GET /api/servers/:sid
pub async fn get_server_by_id(
    State(db): State<DbState>,
    Path(sid): Path<String>,
) -> Result<Json<Server>, (StatusCode, String)> {
    let conn = db.lock().unwrap();
    let mut stmt = conn.prepare("SELECT id, server_id, server_name, icon_url, banner_url, invite_link, members, description, is_verified, is_nsfw, owner, owner_id, added_on, last_bumped FROM servers WHERE server_id = ?")
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let server = stmt
        .query_row([sid], |row| {
            Ok(Server {
                id: row.get(0)?,
                server_id: row.get(1)?,
                server_name: row.get(2)?,
                icon_url: row.get(3)?,
                banner_url: row.get(4)?,
                invite_link: row.get(5)?,
                members: row.get(6)?,
                description: row.get(7)?,
                is_verified: row.get(8)?,
                is_nsfw: row.get(9)?,
                owner: row.get(10)?,
                owner_id: row.get(11)?,
                added_on: row.get(12)?,
                last_bumped: row.get(13)?,
            })
        })
        .optional()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match server {
        Some(s) => Ok(Json(s)),
        None => Err((StatusCode::NOT_FOUND, "Not found".to_string())),
    }
}

// POST /api/servers
pub async fn sync_server(
    State(db): State<DbState>,
    headers: HeaderMap,
    Json(payload): Json<SyncServerInput>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    authorize_request(&headers)?;

    let conn = db.lock().unwrap();
    check_if_banned(&conn, &payload.server_id)?;

    let res = conn.execute(
        "INSERT INTO servers (server_id, server_name, icon_url, banner_url, invite_link, members, description, is_nsfw, owner, owner_id)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(server_id) DO UPDATE SET
             server_name=excluded.server_name,
             icon_url=excluded.icon_url,
             banner_url=excluded.banner_url,
             description=excluded.description,
             members=excluded.members,
             is_nsfw=COALESCE(excluded.is_nsfw, servers.is_nsfw),
             invite_link=excluded.invite_link,
             owner=excluded.owner,
             owner_id=excluded.owner_id",
        params![
            payload.server_id,
            payload.server_name,
            payload.icon_url,
            payload.banner_url,
            payload.invite_link,
            payload.members.unwrap_or(0),
            payload.description,
            payload.is_nsfw.unwrap_or(0),
            payload.owner,
            payload.owner_id
        ],
    );

    match res {
        Ok(_) => Ok(Json(
            serde_json::json!({ "message": "Server listing synced successfully." }),
        )),
        Err(e) => Err((StatusCode::BAD_REQUEST, e.to_string())),
    }
}

// POST /api/servers/:sid/bump
pub async fn bump_server(
    State(db): State<DbState>,
    headers: HeaderMap,
    Path(sid): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    authorize_request(&headers)?;

    let conn = db.lock().unwrap();
    check_if_banned(&conn, &sid)?;

    let last_bumped_str: Option<String> = conn
        .query_row(
            "SELECT last_bumped FROM servers WHERE server_id = ?",
            [sid.clone()],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|e: rusqlite::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let last_bumped_str = match last_bumped_str {
        Some(s) => s,
        None => {
            return Err((
                StatusCode::NOT_FOUND,
                "Server not found in discovery.".to_string(),
            ));
        }
    };

    let format_str = "%Y-%m-%d %H:%M:%S";
    let last_bump = chrono::NaiveDateTime::parse_from_str(&last_bumped_str, format_str)
        .map(|dt| dt.and_utc())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let now = Utc::now();
    let duration = now.signed_duration_since(last_bump);
    let two_hours_in_ms = 2 * 60 * 60 * 1000;

    if duration.num_milliseconds() < two_hours_in_ms {
        let remaining_minutes = ((two_hours_in_ms - duration.num_milliseconds()) as f64
            / (1000.0 * 60.0))
            .ceil() as i64;
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            format!(
                "Slow down! You can bump again in {} minutes.",
                remaining_minutes
            ),
        ));
    }

    let timestamp = now.format(format_str).to_string();
    conn.execute(
        "UPDATE servers SET last_bumped = ? WHERE server_id = ?",
        params![timestamp, sid],
    )
    .map_err(|e: rusqlite::Error| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(
        serde_json::json!({ "message": "Server bumped to the top!" }),
    ))
}

// PATCH /api/servers/:sid
pub async fn patch_server(
    State(db): State<DbState>,
    headers: HeaderMap,
    Path(sid): Path<String>,
    Json(payload): Json<PatchServerInput>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    authorize_request(&headers)?;

    let conn = db.lock().unwrap();
    check_if_banned(&conn, &sid)?;

    let res = conn.execute(
        "UPDATE servers 
         SET is_verified = COALESCE(?, is_verified),
             is_nsfw = COALESCE(?, is_nsfw),
             description = COALESCE(?, description)
         WHERE server_id = ?",
        params![
            payload.is_verified,
            payload.is_nsfw,
            payload.description,
            sid
        ],
    );

    match res {
        Ok(_) => Ok(Json(
            serde_json::json!({ "message": "Updated successfully." }),
        )),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}
