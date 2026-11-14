-- MY PROFILE: the member's own personality, read by their own CLI from their own diary, one
-- reading per finished month. Nothing here ever reaches the chat room.
-- month is the first day of the month read. grp of a score: headline (the three tiles), think,
-- values, shadow. grp of a note: think, values, shadow, patterns, changes.

CREATE TABLE profile_readings (
  id            BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  month         DATE         NOT NULL,
  mbti          VARCHAR(8)   NULL,
  enneagram     VARCHAR(8)   NULL,
  tritype       VARCHAR(8)   NULL,
  headline      TEXT         NULL,
  entries_read  INT UNSIGNED NULL,
  from_date     DATE         NULL,
  to_date       DATE         NULL,
  scanned_at    DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE KEY uq_profile_month (month)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE profile_scores (
  id            BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  reading_id    BIGINT UNSIGNED NOT NULL,
  grp           VARCHAR(10)  NOT NULL,
  label         VARCHAR(40)  NOT NULL,
  score         TINYINT UNSIGNED NOT NULL,
  sort          INT UNSIGNED NOT NULL DEFAULT 0,
  UNIQUE KEY uq_profile_score (reading_id, grp, label),
  CONSTRAINT fk_profile_score_reading FOREIGN KEY (reading_id) REFERENCES profile_readings (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE profile_notes (
  id            BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  reading_id    BIGINT UNSIGNED NOT NULL,
  grp           VARCHAR(10)  NOT NULL,
  line          VARCHAR(240) NOT NULL,
  sort          INT UNSIGNED NOT NULL DEFAULT 0,
  KEY ix_profile_note (reading_id, grp, sort),
  CONSTRAINT fk_profile_note_reading FOREIGN KEY (reading_id) REFERENCES profile_readings (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci
