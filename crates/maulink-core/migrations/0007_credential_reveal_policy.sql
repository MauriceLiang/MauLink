CREATE TABLE credential_reveal_policy (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    mode TEXT NOT NULL CHECK (mode IN ('deny', 'protected', 'direct')),
    password_hash TEXT,
    failed_attempts INTEGER NOT NULL DEFAULT 0 CHECK (failed_attempts >= 0),
    locked_until_ms INTEGER,
    revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
    CHECK (
        (mode = 'protected' AND password_hash IS NOT NULL)
        OR (mode IN ('deny', 'direct') AND password_hash IS NULL)
    )
);

INSERT INTO credential_reveal_policy (singleton, mode)
VALUES (1, 'deny');
