-- Renames the confusingly named network BPS columns to reflect they store cumulative totals.
ALTER TABLE performance_metrics RENAME COLUMN network_rx_bps TO network_rx_cumulative;
ALTER TABLE performance_metrics RENAME COLUMN network_tx_bps TO network_tx_cumulative;