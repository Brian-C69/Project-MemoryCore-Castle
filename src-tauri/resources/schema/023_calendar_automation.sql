ALTER TABLE calendar_events ADD COLUMN automation_lead_min INT NULL AFTER automation_job_id;
CREATE TABLE calendar_triggers (
 id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
 event_id BIGINT UNSIGNED NOT NULL,
 occurrence_start DATETIME NOT NULL,
 job_id BIGINT UNSIGNED NOT NULL,
 due_at DATETIME NOT NULL,
 status VARCHAR(16) NOT NULL,
 note VARCHAR(400) NOT NULL DEFAULT '',
 run_id BIGINT UNSIGNED NULL,
 claimed_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
 UNIQUE KEY ux_calendar_trigger (event_id, occurrence_start),
 KEY ix_calendar_trigger_claimed (claimed_at),
 CONSTRAINT fk_calendar_trigger_event FOREIGN KEY (event_id) REFERENCES calendar_events (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
