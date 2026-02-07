-- Add start/finish dates, notes, rewatching, and rewatch count to library entries.
ALTER TABLE library_entry ADD COLUMN start_date TEXT;
ALTER TABLE library_entry ADD COLUMN finish_date TEXT;
ALTER TABLE library_entry ADD COLUMN notes TEXT;
ALTER TABLE library_entry ADD COLUMN rewatching INTEGER NOT NULL DEFAULT 0;
ALTER TABLE library_entry ADD COLUMN rewatch_count INTEGER NOT NULL DEFAULT 0;
