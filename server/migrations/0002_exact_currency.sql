ALTER TABLE parts
    ADD COLUMN unit_cost_cents BIGINT;

UPDATE parts
SET unit_cost_cents = ROUND(unit_cost * 100)::BIGINT;

ALTER TABLE parts
    ALTER COLUMN unit_cost_cents SET DEFAULT 0,
    ALTER COLUMN unit_cost_cents SET NOT NULL,
    ADD CONSTRAINT parts_unit_cost_cents_nonnegative CHECK (unit_cost_cents >= 0);

ALTER TABLE parts
    DROP COLUMN unit_cost;
