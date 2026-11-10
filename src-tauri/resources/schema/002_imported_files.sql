-- The safety net of moving in: every markdown file Castle deletes after an import is kept
-- here first, exactly as it was, so Export can always write the original back.

CREATE TABLE imported_files (
  path          VARCHAR(500) NOT NULL PRIMARY KEY,
  body          MEDIUMTEXT   NOT NULL,
  sha256        CHAR(64)     NOT NULL,
  imported_at   DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
