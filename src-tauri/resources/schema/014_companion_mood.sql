CREATE TABLE companion_mood (
  id         BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  mood_date  DATE          NOT NULL,
  words      VARCHAR(400)  NOT NULL,
  tone       VARCHAR(300)  NOT NULL,
  source     VARCHAR(12)   NOT NULL,
  lean       VARCHAR(120)  NULL,
  why        VARCHAR(300)  NULL,
  created_at DATETIME      NOT NULL DEFAULT CURRENT_TIMESTAMP,
  KEY ix_mood_date (mood_date, id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- The companion's mood engine: one row per mood. A day's mood is the newest row with that date.
-- source: daily (written by the member's own CLI from a random lean at the day's first start),
-- companion (changed by the companion itself through castle-memory mood set, with its why),
-- fallback (made by Castle from the palette when the CLI could not be reached).
-- words is comma separated, tone is one sentence. Every row is kept as the mood's history.
