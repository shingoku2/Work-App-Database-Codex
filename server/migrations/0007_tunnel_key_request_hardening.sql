ALTER TABLE tunnel_key_requests
  ADD COLUMN status_token TEXT NOT NULL DEFAULT gen_random_uuid()::text;

ALTER TABLE tunnel_key_requests
  DROP CONSTRAINT tunnel_key_requests_status_check;
ALTER TABLE tunnel_key_requests
  ADD CONSTRAINT tunnel_key_requests_status_check
  CHECK (status IN ('pending', 'approved', 'rejected', 'revoked'));

CREATE UNIQUE INDEX tunnel_key_requests_status_token_idx
  ON tunnel_key_requests (status_token);
