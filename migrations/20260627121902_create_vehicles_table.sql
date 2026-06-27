CREATE TABLE vehicles (
    id UUID PRIMARY KEY,
    vin TEXT NOT NULL UNIQUE,
    model TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);