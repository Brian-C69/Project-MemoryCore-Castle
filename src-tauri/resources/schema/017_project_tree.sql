ALTER TABLE projects
  ADD COLUMN parent_id BIGINT UNSIGNED NULL,
  ADD KEY ix_projects_parent (parent_id),
  ADD CONSTRAINT fk_project_parent FOREIGN KEY (parent_id) REFERENCES projects (id) ON DELETE SET NULL;

CREATE TABLE project_tasks (
  id            BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  project_id    BIGINT UNSIGNED NOT NULL,
  title         VARCHAR(300) NOT NULL,
  done_at       DATETIME     NULL,
  due_date      DATE         NULL,
  position      INT UNSIGNED NOT NULL DEFAULT 0,
  reminder_id   BIGINT UNSIGNED NULL,
  created_at    DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
  KEY ix_project_tasks_project (project_id, position, id),
  CONSTRAINT fk_project_task_project FOREIGN KEY (project_id) REFERENCES projects (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- Sub-projects and tasks. A project can sit under one parent (projects.parent_id), one level
-- deep: a parent is never itself a sub-project. Each sub-project keeps its own shelf, so one
-- ticket can rest while its parent runs. A parent removed for good leaves its sub-projects
-- standing on their own.
-- project_tasks: a project's to-do list in the member's order (position). done_at is set when
-- it is ticked. A task with a due date may raise a reminder (reminder_id), closed with it.
