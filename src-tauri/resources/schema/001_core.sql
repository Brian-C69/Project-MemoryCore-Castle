-- MemoryCore Castle - the member's memory, core tables.
-- A migration file is history: never edited once released. A change is a new file.

CREATE TABLE diary_entries (
  id            BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  entry_date    DATE         NULL,
  session_no    INT UNSIGNED NOT NULL DEFAULT 1,
  title         VARCHAR(255) NOT NULL,
  body          MEDIUMTEXT   NOT NULL,
  source_file   VARCHAR(500) NULL,
  created_at    DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at    DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  UNIQUE KEY uq_diary_source (source_file, session_no),
  KEY ix_diary_date (entry_date, session_no)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE reminders (
  id            BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  title         VARCHAR(255) NOT NULL,
  due_date      DATE         NULL,
  detail        TEXT         NOT NULL,
  state         VARCHAR(16)  NOT NULL DEFAULT 'open',
  position      INT UNSIGNED NOT NULL DEFAULT 0,
  created_at    DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
  done_at       DATETIME     NULL,
  KEY ix_reminders_state (state, position)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE projects (
  id            BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  name          VARCHAR(255) NOT NULL,
  status        VARCHAR(64)  NULL,
  shelf         VARCHAR(16)  NOT NULL DEFAULT 'active',
  summary       TEXT         NULL,
  body          MEDIUMTEXT   NULL,
  source_file   VARCHAR(500) NULL,
  last_updated  VARCHAR(64)  NULL,
  created_at    DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at    DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  UNIQUE KEY uq_projects_name (name)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE link_terminals (
  name          VARCHAR(40)  NOT NULL PRIMARY KEY,
  hosted        TINYINT(1)   NOT NULL DEFAULT 0,
  detail        VARCHAR(255) NULL,
  last_seen     DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE link_notes (
  id            BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  from_terminal VARCHAR(40)  NOT NULL,
  to_terminal   VARCHAR(40)  NOT NULL,
  sent_at       DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
  wake          TINYINT(1)   NOT NULL DEFAULT 1,
  word          TEXT         NULL,
  about         TEXT         NOT NULL,
  reply_to      VARCHAR(40)  NULL,
  context       MEDIUMTEXT   NULL,
  read_at       DATETIME     NULL,
  KEY ix_link_waiting (to_terminal, read_at, id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
