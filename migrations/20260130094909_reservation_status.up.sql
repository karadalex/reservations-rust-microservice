-- Add up migration script here
ALTER TABLE reservations
ADD COLUMN is_active INTEGER NOT NULL DEFAULT 1;

ALTER TABLE reservations
ADD created_at TEXT;

ALTER TABLE reservations
ADD updated_at TEXT;