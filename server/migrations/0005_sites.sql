CREATE TABLE sites (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    code TEXT NOT NULL UNIQUE,
    description TEXT,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    version BIGINT NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO sites (name, code, description)
VALUES ('DEP_TX05', 'DEP_TX05', 'Default George West fleet site');

ALTER TABLE users ADD COLUMN site_id BIGINT REFERENCES sites(id) ON DELETE SET NULL;
ALTER TABLE miners ADD COLUMN site_id BIGINT REFERENCES sites(id) ON DELETE RESTRICT;
ALTER TABLE parts ADD COLUMN site_id BIGINT REFERENCES sites(id) ON DELETE RESTRICT;

UPDATE miners SET site_id = (SELECT id FROM sites WHERE code = 'DEP_TX05') WHERE site_id IS NULL;
UPDATE parts SET site_id = (SELECT id FROM sites WHERE code = 'DEP_TX05') WHERE site_id IS NULL;

ALTER TABLE miners ALTER COLUMN site_id SET NOT NULL;
ALTER TABLE parts ALTER COLUMN site_id SET NOT NULL;

DROP INDEX IF EXISTS idx_miners_status;
ALTER TABLE miners DROP CONSTRAINT IF EXISTS miners_serial_key;
CREATE UNIQUE INDEX idx_miners_site_serial ON miners(site_id, serial);
CREATE INDEX idx_miners_site_status ON miners(site_id, status);
CREATE INDEX idx_parts_site ON parts(site_id);
CREATE INDEX idx_users_site ON users(site_id);
CREATE INDEX idx_sites_enabled ON sites(enabled);
