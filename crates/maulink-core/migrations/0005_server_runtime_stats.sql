CREATE TABLE server_runtime_stats (
    server_id TEXT PRIMARY KEY REFERENCES servers(id) ON DELETE CASCADE,
    last_success_at_ms INTEGER,
    last_failure_at_ms INTEGER,
    last_preflight_at_ms INTEGER,
    last_preflight_latency_ms INTEGER CHECK
        (last_preflight_latency_ms IS NULL OR last_preflight_latency_ms >= 0),
    last_failure_code TEXT,
    updated_at_ms INTEGER NOT NULL,
    CHECK ((last_failure_at_ms IS NULL) = (last_failure_code IS NULL))
);
