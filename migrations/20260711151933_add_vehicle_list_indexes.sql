CREATE INDEX idx_vehicles_status
ON vehicles (status);

CREATE INDEX idx_vehicles_created_at_id
ON vehicles (created_at DESC, id DESC);