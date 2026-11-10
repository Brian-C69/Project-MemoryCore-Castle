-- The memory architecture itself (master memory, main memory, session memory) and the skills.
-- Memory files live only here. Skills live here AND as files on disk (the CLI finds a skill
-- only as a file), kept the same by Castle.

CREATE TABLE memory_files (
  path          VARCHAR(300) NOT NULL PRIMARY KEY,
  body          MEDIUMTEXT   NOT NULL,
  boot_order    INT UNSIGNED NULL,
  updated_at    DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE memory_file_history (
  id            BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  path          VARCHAR(300) NOT NULL,
  body          MEDIUMTEXT   NOT NULL,
  replaced_at   DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
  KEY ix_memory_history (path, id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE session_sections (
  id            BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  file          VARCHAR(300) NOT NULL,
  position      INT UNSIGNED NOT NULL,
  heading       VARCHAR(255) NOT NULL,
  body          MEDIUMTEXT   NOT NULL,
  updated_at    DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  UNIQUE KEY uq_session_heading (heading)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE session_section_history (
  id            BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  heading       VARCHAR(255) NOT NULL,
  body          MEDIUMTEXT   NOT NULL,
  replaced_at   DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
  KEY ix_session_history (heading, id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE skills (
  plugin        VARCHAR(100) NOT NULL,
  name          VARCHAR(100) NOT NULL,
  description   TEXT         NULL,
  updated_at    DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  PRIMARY KEY (plugin, name)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE skill_files (
  plugin        VARCHAR(100) NOT NULL,
  name          VARCHAR(100) NOT NULL,
  path          VARCHAR(300) NOT NULL,
  body          MEDIUMTEXT   NOT NULL,
  updated_at    DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (plugin, name, path)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE skill_file_history (
  id            BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  plugin        VARCHAR(100) NOT NULL,
  name          VARCHAR(100) NOT NULL,
  path          VARCHAR(300) NOT NULL,
  body          MEDIUMTEXT   NOT NULL,
  replaced_at   DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
  KEY ix_skill_history (plugin, name, path, id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
