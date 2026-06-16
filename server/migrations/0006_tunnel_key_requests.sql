CREATE TABLE tunnel_key_requests (
    id          BIGSERIAL PRIMARY KEY,
    label       TEXT NOT NULL,
    public_key  TEXT NOT NULL,
    status      TEXT NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending', 'approved', 'rejected')),
    note        TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX tunnel_key_requests_status_idx ON tunnel_key_requests (status);
CREATE INDEX tunnel_key_requests_created_at_idx ON tunnel_key_requests (created_at);
