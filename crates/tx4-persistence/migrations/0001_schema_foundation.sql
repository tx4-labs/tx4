-- Phase-1C migration foundation marker.
-- Proves ordered SQLx discovery/execution only.
-- No business tables (transactions, payments, ledger, outbox, etc.).

CREATE SCHEMA IF NOT EXISTS tx4_infra;

COMMENT ON SCHEMA tx4_infra IS
  'TX4 infrastructure schema namespace established by Phase-1C; business tables are not authorized here.';
