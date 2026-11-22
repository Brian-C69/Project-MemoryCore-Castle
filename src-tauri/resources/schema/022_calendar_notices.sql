CREATE TABLE calendar_notices (
 id BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
 event_id BIGINT UNSIGNED NOT NULL,
 occurrence_start DATETIME NOT NULL,
 due_at DATETIME NOT NULL,
 title VARCHAR(160) NOT NULL,
 lead_min INT NOT NULL,
 late TINYINT NOT NULL DEFAULT 0,
 hidden TINYINT NOT NULL DEFAULT 0,
 sent_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
 dismissed_at DATETIME NULL,
 UNIQUE KEY ux_calendar_notice (event_id, occurrence_start),
 KEY ix_calendar_notice_sent (sent_at),
 CONSTRAINT fk_calendar_notice_event FOREIGN KEY (event_id) REFERENCES calendar_events (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
