-- Approved plans for the companion's project folders (the DEV tab and `castle-memory plan`).
-- A plan is kept whole in project_plans; each `- [ ]` box and each `## Phase N` heading is one
-- row of project_plan_items, in the order the plan wrote them.
-- folder is the project folder as given; folder_key is the same path made comparable (no
-- trailing slash, lowercase on Windows). plan_key is the SHA-256 of folder_key and the plan's
-- text, so the same plan copied twice into the same folder is one row.

CREATE TABLE project_plans (
  id            BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  folder        VARCHAR(500) NOT NULL,
  folder_key    VARCHAR(500) NOT NULL,
  title         VARCHAR(500) NOT NULL,
  phase         VARCHAR(100) NULL,
  status        VARCHAR(20)  NOT NULL DEFAULT 'open',
  approved_at   DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
  body          MEDIUMTEXT   NOT NULL,
  plan_key      CHAR(64)     NOT NULL,
  source_file   VARCHAR(500) NULL,
  created_at    DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at    DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  UNIQUE KEY uq_plan_key (plan_key),
  KEY ix_plan_folder (folder_key(191), status, id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE project_plan_items (
  id            BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  plan_id       BIGINT UNSIGNED NOT NULL,
  ordinal       INT UNSIGNED NOT NULL,
  kind          VARCHAR(10)  NOT NULL DEFAULT 'task',
  text          TEXT         NOT NULL,
  state         VARCHAR(10)  NOT NULL DEFAULT 'open',
  done_at       DATETIME     NULL,
  note          VARCHAR(500) NULL,
  UNIQUE KEY uq_plan_item (plan_id, ordinal),
  CONSTRAINT fk_plan_item_plan FOREIGN KEY (plan_id) REFERENCES project_plans (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
