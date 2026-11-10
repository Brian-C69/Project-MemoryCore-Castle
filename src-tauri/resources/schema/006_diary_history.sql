-- A diary entry that is rewritten keeps its older words here.

CREATE TABLE diary_entry_history (
  id            BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  entry_id      BIGINT UNSIGNED NOT NULL,
  title         VARCHAR(255) NOT NULL,
  body          MEDIUMTEXT   NOT NULL,
  replaced_at   DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
  KEY ix_diary_history (entry_id, id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
