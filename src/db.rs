use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub type DbState = Arc<Mutex<Connection>>;

pub fn init_db() -> Connection {
    let conn = Connection::open("discovery.db").expect("Failed to open database");
    // For server listings
    conn.execute(
        "CREATE TABLE IF NOT EXISTS servers (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            server_id TEXT UNIQUE, 
            server_name TEXT NOT NULL,
            icon_url TEXT,
            banner_url TEXT,
            invite_link TEXT UNIQUE,
            members INTEGER DEFAULT 0,
            description TEXT,
            is_verified INTEGER DEFAULT 0,
            is_nsfw INTEGER DEFAULT 0,
            is_banned INTEGER DEFAULT 0,
            owner TEXT,
            owner_id TEXT NOT NULL,
            added_on DATETIME DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%S', 'now')),
            last_bumped DATETIME DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%S', 'now'))
        )",
        [],
    )
    .expect("Failed to create servers table");
    // For bot listings (Planned)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS bots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            bot_id TEXT UNIQUE NOT NULL,     
            bot_name TEXT NOT NULL,
            avatar_url TEXT,
            banner_url TEXT,
            prefix TEXT DEFAULT '!',            
            invite_link TEXT,                 
            description TEXT,            
            library TEXT,                      
            is_verified INTEGER DEFAULT 0,
            developer_name TEXT,             
            is_banned INTEGER DEFAULT 0,
            developer_id TEXT NOT NULL,       
            added_on DATETIME DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%S', 'now')),
            last_bumped DATETIME DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%S', 'now'))
        )",
        [],
    )
    .expect("Failed to create bots table");
    // For dashboard users (Planned)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id TEXT UNIQUE NOT NULL,
            username TEXT UNIQUE NOT NULL,
            password TEXT
            registered DATETIME DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%S', 'now')),
            last_login DATETIME,
            is_admin INTEGER DEFAULT 0,
            is_banned INTEGER DEFAULT 0
        )",
        [],
    )
    .expect("Failed to create users table");

    // For dashboard login (Planned)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS pending_auth (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id TEXT UNIQUE NOT NULL,
            username TEXT NOT NULL,
            code TEXT NOT NULL,
            created_at DATETIME DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%S', 'now')),
            expires_at DATETIME DEFAULT (STRFTIME('%Y-%m-%d %H:%M:%S', 'now', '+15 minutes'))
        )",
        [],
    )
    .expect("Failed to create pending_auth table");

    conn
}
