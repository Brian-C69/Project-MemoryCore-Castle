-- A plain name for each memory row ("Identity Core"), for people to read.
-- `path` stays the row's key: the companion's own memory text refers to it, and Export uses it.

ALTER TABLE memory_files ADD COLUMN title VARCHAR(255) NULL;
