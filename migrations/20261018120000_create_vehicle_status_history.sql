CREATE TABLE vehicle_status_history (
    event_id UUID PRIMARY KEY,
    vehicle_id UUID NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('online', 'offline', 'maintenance')),
    occurred_at TIMESTAMPTZ NOT NULL,
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX vehicle_status_history_vehicle_time_idx
    ON vehicle_status_history (vehicle_id, occurred_at, event_id);
