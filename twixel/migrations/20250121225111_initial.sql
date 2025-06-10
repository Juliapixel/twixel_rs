CREATE TABLE IF NOT EXISTS users (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    creation_ts TEXT NOT NULL,
    role TEXT,
    fish_reminder INTEGER NOT NULL DEFAULT 0 CHECK (fish_reminder IN (0, 1))
) STRICT;

CREATE TABLE IF NOT EXISTS twitch_users (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    twitch_id TEXT NOT NULL UNIQUE,
    twitch_login TEXT NOT NULL,
    twitch_display_name TEXT NOT NULL,
    bot_joined INTEGER NOT NULL DEFAULT 0 CHECK (bot_joined IN (0, 1))
) STRICT;

CREATE INDEX IF NOT EXISTS tw_uid ON twitch_users (user_id);

CREATE TABLE IF NOT EXISTS reminded_users (
    user_id INTEGER NOT NULL UNIQUE REFERENCES users (id) ON DELETE CASCADE
) STRICT;
