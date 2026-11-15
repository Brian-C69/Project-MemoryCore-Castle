-- LIBRARY: the member's own books, kept in the memory database instead of markdown. One row per
-- book; shelf is the section it sits on (architecture, pattern, general...), slug its name on the
-- shelf. source_file is where the book came from when it was taken in from library/ (or null).

CREATE TABLE library_books (
  id            BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  shelf         VARCHAR(80)  NOT NULL,
  slug          VARCHAR(120) NOT NULL,
  title         VARCHAR(200) NOT NULL,
  body          MEDIUMTEXT   NOT NULL,
  source_file   VARCHAR(500) NULL,
  created_at    DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at    DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  UNIQUE KEY uq_library_book (shelf, slug)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
