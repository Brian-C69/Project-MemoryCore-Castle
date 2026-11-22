ALTER TABLE calendar_events ADD COLUMN ics_uid VARCHAR(190) NULL AFTER automation_lead_min;
ALTER TABLE calendar_events ADD UNIQUE KEY ux_calendar_ics_uid (ics_uid);
