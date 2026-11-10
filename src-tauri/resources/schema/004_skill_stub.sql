-- A skill can be served as a stub: a small SKILL.md on disk (so the CLI lists and triggers it)
-- whose instructions are loaded from here, by section, when the skill fires.
-- Off by default: a skill stays a whole file until it is switched.

ALTER TABLE skills ADD COLUMN stubbed TINYINT(1) NOT NULL DEFAULT 0;
