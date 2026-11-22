CREATE TABLE calendar_events (
 id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
 title VARCHAR(160) NOT NULL,
 note TEXT NOT NULL,
 starts_at DATETIME NOT NULL,
 ends_at DATETIME NULL,
 all_day TINYINT NOT NULL DEFAULT 0,
 timezone VARCHAR(40) NOT NULL DEFAULT 'local',
 repeat_json JSON NULL,
 remind_before_min INT NULL,
 colour VARCHAR(16) NOT NULL DEFAULT 'accent',
 state VARCHAR(16) NOT NULL DEFAULT 'active',
 created_by VARCHAR(24) NOT NULL DEFAULT 'member',
 automation_job_id BIGINT UNSIGNED NULL,
 created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
 updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
 KEY ix_calendar_events (state, starts_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
CREATE TABLE calendar_exceptions (
 id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
 event_id BIGINT UNSIGNED NOT NULL,
 occurrence_date DATE NOT NULL,
 kind VARCHAR(8) NOT NULL,
 new_starts_at DATETIME NULL,
 UNIQUE KEY ux_calendar_exception (event_id, occurrence_date),
 CONSTRAINT fk_calendar_exception_event FOREIGN KEY (event_id) REFERENCES calendar_events (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
