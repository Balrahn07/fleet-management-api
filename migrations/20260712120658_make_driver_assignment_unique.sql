CREATE UNIQUE INDEX idx_vehicles_unique_driver_id
ON vehicles (driver_id)
WHERE driver_id IS NOT NULL;