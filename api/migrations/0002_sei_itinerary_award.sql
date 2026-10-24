-- Feedback from the TJRR travel desk (SGA), first round.
-- Every column is nullable: rows created before this migration have none of it,
-- and the API requires the values for anything created from now on.

-- Each demand already lives in an SEI process; every printable is attached to it.
ALTER TABLE quotations ADD COLUMN sei_process TEXT;

-- Structured itinerary. From Roraima the cheapest flight often adds a connection
-- and hours of travel, so price alone cannot decide; the TJ weighs duration too.
ALTER TABLE proposals ADD COLUMN airline TEXT;
ALTER TABLE proposals ADD COLUMN departs_at TIMESTAMPTZ;
ALTER TABLE proposals ADD COLUMN duration_minutes INTEGER;
ALTER TABLE proposals ADD COLUMN connections INTEGER;

-- Award decision record: the verified reason, and who was eligible at that
-- moment (certidões expire every ~30 days, so this cannot be recomputed later).
ALTER TABLE quotations ADD COLUMN award_reason TEXT;
ALTER TABLE quotations ADD COLUMN award_snapshot JSONB;
