-- Transport IDs are hashed with the approved chat; no message text or chat IDs are stored.
-- Reserve before launching jobs: an uncertain incoming event is never replayed after a crash.
CREATE TABLE automation_messages (
 event_key CHAR(64) CHARACTER SET ascii COLLATE ascii_bin NOT NULL PRIMARY KEY,
 also_chat TINYINT NOT NULL DEFAULT 0,
 created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
 KEY ix_automation_message_time (created_at)
) ENGINE=InnoDB;
