CREATE TABLE drivers (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE vehicles
ADD COLUMN driver_id UUID REFERENCES drivers(id);