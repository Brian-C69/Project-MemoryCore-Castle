-- APPEARANCE: the companion's look in words, kept in the memory database instead of markdown.
-- appearance_look holds one row (id 1): the look, the art style and what is worn today.
-- appearance_items holds the wardrobe and the hairstyles; slug is the name of the item's
-- picture folder (appearance/wardrobe/<slug>/, appearance/hairstyles/<slug>/), which stays on disk.

CREATE TABLE appearance_look (
  id            TINYINT UNSIGNED NOT NULL PRIMARY KEY,
  words         TEXT         NOT NULL,
  art_style     VARCHAR(120) NOT NULL DEFAULT '',
  outfit_slug   VARCHAR(64)  NULL,
  hair_slug     VARCHAR(64)  NULL,
  updated_at    DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE appearance_items (
  id            BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  kind          VARCHAR(8)   NOT NULL,
  slug          VARCHAR(64)  NOT NULL,
  name          VARCHAR(80)  NOT NULL,
  words         TEXT         NOT NULL,
  created_at    DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at    DATETIME     NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  UNIQUE KEY uq_appearance_item (kind, slug)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
