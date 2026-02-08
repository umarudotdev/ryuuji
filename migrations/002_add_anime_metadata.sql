-- Add metadata columns to anime table for rich display

ALTER TABLE anime ADD COLUMN synopsis TEXT;
ALTER TABLE anime ADD COLUMN genres TEXT;          -- JSON array
ALTER TABLE anime ADD COLUMN media_type TEXT;
ALTER TABLE anime ADD COLUMN airing_status TEXT;
ALTER TABLE anime ADD COLUMN mean_score REAL;
ALTER TABLE anime ADD COLUMN studios TEXT;         -- JSON array
ALTER TABLE anime ADD COLUMN source TEXT;
ALTER TABLE anime ADD COLUMN rating TEXT;
ALTER TABLE anime ADD COLUMN start_date TEXT;
ALTER TABLE anime ADD COLUMN end_date TEXT;
