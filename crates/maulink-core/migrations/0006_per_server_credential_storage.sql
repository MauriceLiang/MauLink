ALTER TABLE servers
    ADD COLUMN require_authentication INTEGER NOT NULL DEFAULT 1
    CHECK (require_authentication IN (0, 1));

UPDATE servers
SET require_authentication = 0
WHERE credential_ref_id IS NULL;

ALTER TABLE credential_refs
    ADD COLUMN storage_backend TEXT NOT NULL DEFAULT 'native'
    CHECK (storage_backend IN ('native', 'database'));

ALTER TABLE credential_refs
    ADD COLUMN secret_ciphertext BLOB
    CHECK (
        (state = 'pending_write' AND secret_ciphertext IS NULL)
        OR (storage_backend = 'native' AND secret_ciphertext IS NULL)
        OR (storage_backend = 'database' AND secret_ciphertext IS NOT NULL)
    );
