-- Add down migration script here
ALTER TABLE reservations
DROP COLUMN is_active;

ALTER TABLE reservations
DROP COLUMN created_at;

ALTER TABLE reservations
DROP COLUMN updated_at;