-- LIBRARY: a book kept from the Spiral Library remembers where it came from (the Spiral ref) and
-- which edition was kept (the book's updatedAt there), so a newer edition can be noticed and one
-- book is never kept twice. Both stay null for the member's own books.

ALTER TABLE library_books ADD COLUMN origin_ref VARCHAR(500) NULL;
ALTER TABLE library_books ADD COLUMN origin_edition VARCHAR(40) NULL;
CREATE INDEX ix_library_origin ON library_books (origin_ref);
