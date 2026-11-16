CREATE TABLE market_run (
  id             BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  market         VARCHAR(16)    NOT NULL,
  run_no         INT UNSIGNED   NOT NULL,
  start_balance  DECIMAL(28,10) NOT NULL,
  balance        DECIMAL(28,10) NOT NULL,
  started_at     DATETIME       NOT NULL,
  ended_at       DATETIME       NULL,
  open_market    VARCHAR(16)    NULL,
  UNIQUE KEY uq_market_run_no (market, run_no),
  UNIQUE KEY uq_market_run_open (open_market)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- The MARKET room: paper money on public prices. Nothing real is ever ordered.
-- open_market and open_symbol are set by Castle while a run or a position is open and cleared
-- when it ends (a plain column, not a generated one, so a backup restores as it was written).

CREATE TABLE market_ledger (
  id             BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  run_id         BIGINT UNSIGNED NOT NULL,
  kind           VARCHAR(16)    NOT NULL,
  amount         DECIMAL(28,10) NOT NULL,
  balance_after  DECIMAL(28,10) NOT NULL,
  symbol         VARCHAR(32)    NULL,
  note           VARCHAR(200)   NULL,
  created_at     DATETIME       NOT NULL,
  KEY ix_market_ledger_run (run_id, id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE market_order (
  id              BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  run_id          BIGINT UNSIGNED NOT NULL,
  client_order_id VARCHAR(64)    NOT NULL,
  source          VARCHAR(24)    NOT NULL,
  symbol          VARCHAR(32)    NOT NULL,
  side            VARCHAR(8)     NOT NULL,
  order_type      VARCHAR(8)     NOT NULL,
  reduce_only     TINYINT(1)     NOT NULL DEFAULT 0,
  margin_mode     VARCHAR(10)    NULL,
  leverage        DECIMAL(10,4)  NULL,
  qty             DECIMAL(28,10) NOT NULL,
  price           DECIMAL(28,10) NULL,
  reserved_cost   DECIMAL(28,10) NOT NULL DEFAULT 0,
  tp_price        DECIMAL(28,10) NULL,
  sl_price        DECIMAL(28,10) NULL,
  status          VARCHAR(12)    NOT NULL,
  note            VARCHAR(200)   NULL,
  created_at      DATETIME       NOT NULL,
  finished_at     DATETIME       NULL,
  UNIQUE KEY uq_market_order_client (client_order_id),
  KEY ix_market_order_run (run_id, status, id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE market_position (
  id             BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  run_id         BIGINT UNSIGNED NOT NULL,
  symbol         VARCHAR(32)    NOT NULL,
  side           VARCHAR(8)     NOT NULL,
  margin_mode    VARCHAR(10)    NULL,
  leverage       DECIMAL(10,4)  NULL,
  qty            DECIMAL(28,10) NOT NULL,
  entry_price    DECIMAL(28,10) NOT NULL,
  extra_margin   DECIMAL(28,10) NOT NULL DEFAULT 0,
  tp_price       DECIMAL(28,10) NULL,
  sl_price       DECIMAL(28,10) NULL,
  realised_pnl   DECIMAL(28,10) NOT NULL DEFAULT 0,
  fees_paid      DECIMAL(28,10) NOT NULL DEFAULT 0,
  funding_paid   DECIMAL(28,10) NOT NULL DEFAULT 0,
  opened_by      VARCHAR(24)    NOT NULL,
  opened_at      DATETIME       NOT NULL,
  closed_at      DATETIME       NULL,
  open_symbol    VARCHAR(32)    NULL,
  UNIQUE KEY uq_market_position_open (run_id, open_symbol),
  KEY ix_market_position_run (run_id, id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE market_fill (
  id             BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  run_id         BIGINT UNSIGNED NOT NULL,
  order_id       BIGINT UNSIGNED NULL,
  position_id    BIGINT UNSIGNED NOT NULL,
  symbol         VARCHAR(32)    NOT NULL,
  side           VARCHAR(8)     NOT NULL,
  qty            DECIMAL(28,10) NOT NULL,
  price          DECIMAL(28,10) NOT NULL,
  fee            DECIMAL(28,10) NOT NULL,
  fee_role       VARCHAR(8)     NOT NULL,
  kind           VARCHAR(16)    NOT NULL,
  realised_pnl   DECIMAL(28,10) NOT NULL DEFAULT 0,
  bid            DECIMAL(28,10) NULL,
  ask            DECIMAL(28,10) NULL,
  mark           DECIMAL(28,10) NULL,
  feed_age_ms    BIGINT         NULL,
  created_at     DATETIME       NOT NULL,
  KEY ix_market_fill_run (run_id, id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE market_funding (
  id             BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  run_id         BIGINT UNSIGNED NOT NULL,
  position_id    BIGINT UNSIGNED NOT NULL,
  symbol         VARCHAR(32)    NOT NULL,
  funding_time   BIGINT         NOT NULL,
  rate           DECIMAL(28,10) NOT NULL,
  mark           DECIMAL(28,10) NOT NULL,
  amount         DECIMAL(28,10) NOT NULL,
  created_at     DATETIME       NOT NULL,
  UNIQUE KEY uq_market_funding_once (position_id, funding_time),
  KEY ix_market_funding_run (run_id, id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE market_instrument (
  id             BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  market         VARCHAR(16)    NOT NULL,
  symbol         VARCHAR(32)    NOT NULL,
  margin_mode    VARCHAR(10)    NOT NULL DEFAULT 'CROSS',
  leverage       DECIMAL(10,4)  NOT NULL DEFAULT 10,
  rules_json     TEXT           NULL,
  tiers_json     TEXT           NULL,
  refreshed_at   DATETIME       NULL,
  UNIQUE KEY uq_market_instrument (market, symbol)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE market_setting (
  market              VARCHAR(16)    NOT NULL PRIMARY KEY,
  taker_rate          DECIMAL(12,8)  NOT NULL,
  maker_rate          DECIMAL(12,8)  NOT NULL,
  feed_id             VARCHAR(16)    NOT NULL DEFAULT 'BYBIT',
  auto_on             TINYINT(1)     NOT NULL DEFAULT 0,
  auto_every_minutes  INT UNSIGNED   NOT NULL DEFAULT 15,
  auto_round_cap      INT UNSIGNED   NOT NULL DEFAULT 20,
  auto_daily_loss     DECIMAL(28,10) NOT NULL DEFAULT 500,
  auto_daily_target   DECIMAL(28,10) NULL,
  auto_balance_target DECIMAL(28,10) NULL,
  updated_at          DATETIME       NOT NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE market_feed_state (
  market             VARCHAR(16)  NOT NULL PRIMARY KEY,
  status             VARCHAR(12)  NOT NULL,
  status_since       DATETIME     NOT NULL,
  last_evaluated_ms  BIGINT       NULL,
  note               VARCHAR(200) NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE market_turn (
  id              BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  run_id          BIGINT UNSIGNED NOT NULL,
  client_turn_id  VARCHAR(64)  NOT NULL,
  trigger_kind    VARCHAR(12)  NOT NULL,
  cli             VARCHAR(16)  NOT NULL,
  message         TEXT         NULL,
  reply           TEXT         NULL,
  desk_json       MEDIUMTEXT   NULL,
  actions_json    TEXT         NULL,
  duration_ms     BIGINT       NULL,
  status          VARCHAR(12)  NOT NULL,
  created_at      DATETIME     NOT NULL,
  UNIQUE KEY uq_market_turn_client (client_turn_id),
  KEY ix_market_turn_run (run_id, id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE market_lesson (
  id             BIGINT UNSIGNED NOT NULL AUTO_INCREMENT PRIMARY KEY,
  run_id         BIGINT UNSIGNED NOT NULL,
  position_id    BIGINT UNSIGNED NOT NULL,
  symbol         VARCHAR(32)   NOT NULL,
  side           VARCHAR(8)    NOT NULL,
  result_pct     DECIMAL(12,4) NOT NULL,
  traded_by      VARCHAR(24)   NOT NULL,
  lesson         VARCHAR(400)  NOT NULL,
  created_at     DATETIME      NOT NULL,
  UNIQUE KEY uq_market_lesson_position (position_id),
  KEY ix_market_lesson_new (id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

INSERT INTO market_setting (market, taker_rate, maker_rate, updated_at) VALUES ('CRYPTO', 0.00055000, 0.00020000, UTC_TIMESTAMP());

INSERT INTO market_feed_state (market, status, status_since) VALUES ('CRYPTO', 'OK', UTC_TIMESTAMP());
