ALTER TABLE servers ADD COLUMN jump_host TEXT;
ALTER TABLE servers ADD COLUMN jump_port INTEGER NOT NULL DEFAULT 22
    CHECK (jump_port BETWEEN 1 AND 65535);
ALTER TABLE servers ADD COLUMN proxy_type TEXT
    CHECK (proxy_type IS NULL OR proxy_type IN ('socks5', 'http_connect'));
ALTER TABLE servers ADD COLUMN proxy_host TEXT;
ALTER TABLE servers ADD COLUMN proxy_port INTEGER
    CHECK (proxy_port IS NULL OR proxy_port BETWEEN 1 AND 65535);
