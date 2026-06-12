CREATE TABLE audit_log (
    id          BIGSERIAL PRIMARY KEY,
    user_id     BIGINT REFERENCES users(id) ON DELETE SET NULL,
    username    TEXT,
    action      TEXT NOT NULL,
    target_type TEXT,
    target_id   TEXT,
    target_serial TEXT,
    old_values  JSONB,
    new_values  JSONB,
    ip_address  TEXT,
    user_agent  TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX audit_log_user_id_idx   ON audit_log (user_id);
CREATE INDEX audit_log_action_idx    ON audit_log (action);
CREATE INDEX audit_log_created_at_idx ON audit_log (created_at);
