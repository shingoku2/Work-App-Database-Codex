PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS miners (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  serial TEXT NOT NULL UNIQUE,
  model TEXT NOT NULL CHECK (model IN ('S21', 'S21+', 'S21 Pro', 'S21 XP')),
  firmware TEXT,
  location TEXT,
  status TEXT NOT NULL CHECK (status IN ('In Service', 'Under Repair', 'RMA', 'Retired', 'Spare')),
  acquired_date TEXT,
  notes TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS parts (
  sku TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  category TEXT NOT NULL CHECK (category IN ('Hashboard', 'Control Board', 'PSU', 'Fan', 'Cable', 'Misc')),
  qty_on_hand INTEGER NOT NULL DEFAULT 0 CHECK (qty_on_hand >= 0),
  reorder_threshold INTEGER NOT NULL DEFAULT 0 CHECK (reorder_threshold >= 0),
  supplier TEXT,
  unit_cost REAL NOT NULL DEFAULT 0 CHECK (unit_cost >= 0),
  notes TEXT,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_miners_serial ON miners(serial);
CREATE INDEX IF NOT EXISTS idx_miners_status ON miners(status);
