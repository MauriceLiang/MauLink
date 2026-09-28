CREATE TABLE server_groups (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 64),
    sort_order INTEGER NOT NULL DEFAULT 0,
    revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE TABLE credential_refs (
    id TEXT PRIMARY KEY,
    owner_server_id TEXT NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('password', 'passphrase')),
    state TEXT NOT NULL CHECK (state IN
        ('pending_write', 'active', 'retained', 'pending_delete')),
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE TABLE servers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL CHECK (length(trim(name)) BETWEEN 1 AND 128),
    host TEXT NOT NULL,
    port INTEGER NOT NULL CHECK (port BETWEEN 1 AND 65535),
    username TEXT NOT NULL CHECK (length(username) BETWEEN 1 AND 256),
    auth_type TEXT NOT NULL CHECK (auth_type IN ('password', 'private_key')),
    private_key_path BLOB,
    private_key_path_encoding TEXT CHECK
        (private_key_path_encoding IN ('unix_bytes', 'windows_utf16le')),
    credential_ref_id TEXT REFERENCES credential_refs(id) ON DELETE SET NULL,
    group_id TEXT REFERENCES server_groups(id) ON DELETE SET NULL,
    connect_timeout_ms INTEGER NOT NULL DEFAULT 15000
        CHECK (connect_timeout_ms BETWEEN 1000 AND 120000),
    keepalive_interval_s INTEGER NOT NULL DEFAULT 30
        CHECK (keepalive_interval_s BETWEEN 5 AND 300),
    revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    CHECK ((private_key_path IS NULL) = (private_key_path_encoding IS NULL)),
    CHECK (auth_type <> 'private_key' OR private_key_path IS NOT NULL)
);

CREATE INDEX servers_group_id_idx ON servers(group_id);

CREATE TABLE known_hosts (
    normalized_host TEXT NOT NULL,
    port INTEGER NOT NULL CHECK (port BETWEEN 1 AND 65535),
    key_algorithm TEXT NOT NULL,
    public_key_blob BLOB NOT NULL,
    fingerprint_sha256 TEXT NOT NULL,
    revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
    trusted_at_ms INTEGER NOT NULL,
    PRIMARY KEY (normalized_host, port)
);

CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value_json TEXT NOT NULL,
    revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
    updated_at_ms INTEGER NOT NULL
);
