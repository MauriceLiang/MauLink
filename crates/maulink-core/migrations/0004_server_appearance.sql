CREATE TABLE server_appearance (
    server_id TEXT PRIMARY KEY REFERENCES servers(id) ON DELETE CASCADE,
    label_color TEXT CHECK (label_color IS NULL OR length(label_color) = 7),
    environment TEXT CHECK (environment IS NULL OR environment IN
        ('production', 'staging', 'development', 'custom')),
    terminal_override_enabled INTEGER NOT NULL DEFAULT 0
        CHECK (terminal_override_enabled IN (0, 1)),
    terminal_appearance_json TEXT NOT NULL,
    revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
    updated_at_ms INTEGER NOT NULL
);
