-- A project can rest as well as run. projects.shelf holds where it is: active, paused, archived or
-- deleted. shelf_note says why it moved there (a pause's reason); shelved_at is when it moved, so a
-- deleted project is kept on Recently deleted for 30 days and then removed for good.
ALTER TABLE projects
  ADD COLUMN shelf_note VARCHAR(500) NULL,
  ADD COLUMN shelved_at DATETIME NULL;
