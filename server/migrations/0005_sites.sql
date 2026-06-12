CREATE TABLE sites (
    id          BIGSERIAL PRIMARY KEY,
    name        TEXT NOT NULL,
    code        TEXT NOT NULL UNIQUE,
    description TEXT,
    enabled     BOOLEAN NOT NULL DEFAULT TRUE,
    version     BIGINT NOT NULL DEFAULT 1,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Backfill default site
INSERT INTO sites (name, code, enabled) VALUES ('DEP_TX05', 'DEP_TX05', TRUE);

-- Add site_id to miners (nullable during transition; all existing rows get default site)
ALTER TABLE miners
    ADD COLUMN site_id BIGINT REFERENCES sites(id) ON DELETE RESTRICT;

UPDATE miners SET site_id = (SELECT id FROM sites WHERE code = 'DEP_TX05');

-- Re-create unique constraint scoped to site
ALTER TABLE miners DROP CONSTRAINT IF EXISTS miners_serial_key;
ALTER TABLE miners ALTER COLUMN site_id SET NOT NULL;
ALTER TABLE miners ADD CONSTRAINT miners_site_serial_key UNIQUE (site_id, serial);

-- Add site_id to parts
ALTER TABLE parts
    ADD COLUMN site_id BIGINT REFERENCES sites(id) ON DELETE RESTRICT;

UPDATE parts SET site_id = (SELECT id FROM sites WHERE code = 'DEP_TX05');

ALTER TABLE parts DROP CONSTRAINT IF EXISTS parts_sku_key;
ALTER TABLE parts ALTER COLUMN site_id SET NOT NULL;
ALTER TABLE parts ADD CONSTRAINT parts_site_sku_key UNIQUE (site_id, sku);

-- Add site_id to users (nullable; users not assigned to a specific site see all)
ALTER TABLE users
    ADD COLUMN site_id BIGINT REFERENCES sites(id) ON DELETE SET NULL;
