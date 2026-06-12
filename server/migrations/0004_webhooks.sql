CREATE TABLE webhooks (
    id         BIGSERIAL PRIMARY KEY,
    name       TEXT NOT NULL,
    url        TEXT NOT NULL,
    secret     TEXT,
    events     TEXT[] NOT NULL DEFAULT '{}',
    enabled    BOOLEAN NOT NULL DEFAULT TRUE,
    version    BIGINT NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE webhook_deliveries (
    id              BIGSERIAL PRIMARY KEY,
    webhook_id      BIGINT NOT NULL REFERENCES webhooks(id) ON DELETE CASCADE,
    event           TEXT NOT NULL,
    payload         JSONB NOT NULL,
    response_status INT,
    response_body   TEXT,
    success         BOOLEAN NOT NULL DEFAULT FALSE,
    error           TEXT,
    attempts        INT NOT NULL DEFAULT 1,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    delivered_at    TIMESTAMPTZ
);

CREATE INDEX webhook_deliveries_webhook_id_idx  ON webhook_deliveries (webhook_id);
CREATE INDEX webhook_deliveries_created_at_idx  ON webhook_deliveries (created_at);
