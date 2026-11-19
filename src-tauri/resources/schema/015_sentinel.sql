CREATE TABLE sentinel_findings (
  id           BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  fingerprint  CHAR(40)      NOT NULL,
  kind         VARCHAR(24)   NOT NULL,
  hint         VARCHAR(8)    NOT NULL,
  title        VARCHAR(200)  NOT NULL,
  detail       VARCHAR(1000) NOT NULL,
  fix_move     VARCHAR(24)   NULL,
  fix_arg      VARCHAR(200)  NULL,
  state        VARCHAR(10)   NOT NULL DEFAULT 'open',
  state_why    VARCHAR(400)  NULL,
  first_seen   DATETIME      NOT NULL DEFAULT CURRENT_TIMESTAMP,
  last_seen    DATETIME      NOT NULL DEFAULT CURRENT_TIMESTAMP,
  closed_at    DATETIME      NULL,
  UNIQUE KEY uq_sentinel_findings_print (fingerprint),
  KEY ix_sentinel_findings_state (state, id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- The Sentinel: the member's companion keeps watch over the member's own computer.
-- A finding is marked by what it is (its fingerprint), never by a process id or a time.
-- hint: note, look or alarm. state: open, settled (the companion's judgment, with its reason),
-- accepted (the member's word that it is normal), fixed, or gone.
-- Nothing in these tables ever holds the contents of a member's files.

CREATE TABLE sentinel_known (
  fingerprint  CHAR(40)      NOT NULL PRIMARY KEY,
  kind         VARCHAR(24)   NOT NULL,
  title        VARCHAR(200)  NOT NULL,
  accepted_by  VARCHAR(10)   NOT NULL,
  accepted_at  DATETIME      NOT NULL DEFAULT CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- What the machine already had at the first look (accepted_by baseline), and what the member
-- said is normal (accepted_by member). A known thing opens no finding.

CREATE TABLE sentinel_patrols (
  id           BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  started_at   DATETIME      NOT NULL DEFAULT CURRENT_TIMESTAMP,
  finished_at  DATETIME      NULL,
  cause        VARCHAR(8)    NOT NULL,
  status       VARCHAR(8)    NULL,
  line         VARCHAR(400)  NULL,
  seen         INT UNSIGNED  NOT NULL DEFAULT 0,
  changed      INT UNSIGNED  NOT NULL DEFAULT 0,
  KEY ix_sentinel_patrols_started (started_at, id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- cause: hour, alarm or now. status: quiet, look or alarm, from the companion's verdict line.

CREATE TABLE sentinel_alarms (
  id           BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  raised_at    DATETIME      NOT NULL DEFAULT CURRENT_TIMESTAMP,
  source       VARCHAR(24)   NOT NULL,
  detail       VARCHAR(400)  NOT NULL,
  finding_id   BIGINT UNSIGNED NULL,
  seen_at      DATETIME      NULL,
  KEY ix_sentinel_alarms_seen (seen_at, id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE sentinel_actions (
  id           BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  finding_id   BIGINT UNSIGNED NULL,
  move         VARCHAR(24)   NOT NULL,
  arg          VARCHAR(200)  NULL,
  asked_by     VARCHAR(8)    NOT NULL,
  state        VARCHAR(8)    NOT NULL DEFAULT 'waiting',
  result       VARCHAR(400)  NULL,
  undo_recipe  VARCHAR(1000) NULL,
  asked_at     DATETIME      NOT NULL DEFAULT CURRENT_TIMESTAMP,
  done_at      DATETIME      NULL,
  undone_at    DATETIME      NULL,
  KEY ix_sentinel_actions_finding (finding_id, id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- asked_by: officer (the companion, its own list only), reflex (the watch, firewall and
-- protection only) or member (a spent click). state: waiting, done, failed or undone.

CREATE TABLE sentinel_clicks (
  id           BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  finding_id   BIGINT UNSIGNED NOT NULL,
  word         VARCHAR(8)    NOT NULL,
  clicked_at   DATETIME      NOT NULL DEFAULT CURRENT_TIMESTAMP,
  used_at      DATETIME      NULL,
  KEY ix_sentinel_clicks_finding (finding_id, id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- A click is written by Castle when the member presses Fix, This is normal or Undo. It is fresh
-- for ten minutes and spent once. The companion's own words are never a click.
