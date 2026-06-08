CREATE TABLE users (
    id BIGSERIAL PRIMARY KEY,
    username TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('admin', 'user')),
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    version BIGINT NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE sessions (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    revoked_at TIMESTAMPTZ
);

CREATE INDEX idx_sessions_token_hash ON sessions(token_hash);
CREATE INDEX idx_sessions_expiry ON sessions(expires_at);

CREATE TABLE miners (
    id BIGSERIAL PRIMARY KEY,
    serial TEXT NOT NULL UNIQUE,
    model TEXT NOT NULL CHECK (model IN ('S21', 'S21+', 'S21 Pro', 'S21 XP')),
    firmware TEXT,
    client_name TEXT,
    miner_type TEXT,
    ip_address TEXT,
    mac_address TEXT,
    pickaxe TEXT,
    miner_state TEXT,
    miner_row TEXT,
    miner_index TEXT,
    miner_rack TEXT,
    miner_rack_group TEXT,
    location TEXT,
    status TEXT NOT NULL CHECK (status IN ('In Service', 'Under Repair', 'RMA', 'Retired', 'Spare')),
    acquired_date TEXT,
    notes TEXT,
    version BIGINT NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_miners_status ON miners(status);

CREATE TABLE parts (
    sku TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    category TEXT NOT NULL CHECK (category IN ('Hashboard', 'Control Board', 'PSU', 'Fan', 'Cable', 'Misc')),
    qty_on_hand BIGINT NOT NULL DEFAULT 0 CHECK (qty_on_hand >= 0),
    reorder_threshold BIGINT NOT NULL DEFAULT 0 CHECK (reorder_threshold >= 0),
    supplier TEXT,
    unit_cost DOUBLE PRECISION NOT NULL DEFAULT 0 CHECK (unit_cost >= 0),
    notes TEXT,
    version BIGINT NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
