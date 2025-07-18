# Performance Metrics Data Retention and Downsampling Plan

This document outlines the strategy for managing performance metric data to balance data granularity, query performance, and long-term storage costs. The plan leverages TimescaleDB's native features for an efficient and automated implementation.

## 1. Core Technology

The entire data lifecycle management will be handled by **TimescaleDB**, utilizing its three core features:
- **Hypertables**: To efficiently partition and manage time-series data.
- **Continuous Aggregates**: To automatically downsample raw data into lower-granularity aggregates.
- **Data Retention Policies**: To automatically drop old and expired data chunks.

This approach delegates the heavy lifting to the database layer, minimizing application-level complexity and maximizing performance.

## 2. Data Retention and Granularity Strategy

A four-tiered data pyramid strategy will be implemented:

| Data Tier | Granularity | Retention Period | Primary Use Case |
| :--- | :--- | :--- | :--- |
| **Tier 1: Raw Data** | Second-level | **24 Hours** | Real-time monitoring, immediate incident diagnosis. |
| **Tier 2: Minute Aggregates** | 1 Minute | **7 Days** | Weekly trend analysis, identifying daily patterns. |
| **Tier 3: Hourly Aggregates** | 1 Hour | **30 Days** | Monthly reviews, capacity planning. |
| **Tier 4: Daily Aggregates** | 1 Day | **365 Days** | Yearly trend analysis, long-term capacity planning. |

Data older than 365 days will be permanently deleted.

## 3. Aggregation Function Strategy

To ensure the aggregated data remains representative and insightful, the following "Golden Quad" aggregation strategy will be applied to all fluctuating performance metrics (e.g., `cpu_usage_percent`, `memory_usage_bytes`, `network_rx_instant_bps`):

- **`AVG()`**: To capture the central tendency and draw trend lines.
- **`MAX()`**: To identify peak load and resource pressure.
- **`MIN()`**: To understand idle periods and resource valleys.
- **`approx_percentile(0.95, ...)`**: To calculate the 95th percentile, providing a statistically robust measure of typical performance, ignoring extreme outliers.

For static capacity metrics (e.g., `memory_total_bytes`), `MAX()` will be used to carry the value over.

## 4. Implementation Plan (High-Level)

The implementation will be carried out primarily through a database migration script with the following steps:

1.  **Convert `performance_metrics` and its related tables (`performance_disk_usages`, etc.) to Hypertables** using `create_hypertable()`.
2.  **Create Continuous Aggregate Views**:
    - `performance_metrics_summary_1m` (Minute-level)
    - `performance_metrics_summary_1h` (Hour-level, based on the minute view)
    - `performance_metrics_summary_1d` (Day-level, based on the hour view)
3.  **Apply Policies**:
    - Set up `add_continuous_aggregate_policy()` for each aggregate view to define its automatic refresh schedule.
    - Set up `add_retention_policy()` for the raw hypertable and each aggregate view to enforce the retention periods defined above.
4.  **Refactor Application Code**:
    - Modify the data access layer (e.g., `performance_service.rs`) to intelligently query the appropriate table/view based on the requested time range, thus improving frontend performance.

## 5. Process Flowchart

```mermaid
graph TD
    subgraph "Data Source (Agent)"
        A["Raw Performance Data"] --> B["performance_metrics (Hypertable)"];
    end

    subgraph "TimescaleDB Automated Processing"
        B -- "Auto-Aggregate" --> C["Minute Aggregates (7-day retention)"];
        C -- "Auto-Aggregate" --> D["Hour Aggregates (30-day retention)"];
        D -- "Auto-Aggregate" --> E["Day Aggregates (365-day retention)"];

        B -- "Auto-Drop > 24h" --> X1["(Data Discarded)"];
        C -- "Auto-Drop > 7d" --> X2["(Data Discarded)"];
        D -- "Auto-Drop > 30d" --> X3["(Data Discarded)"];
        E -- "Auto-Drop > 365d" --> X4["(Data Discarded)"];
    end

    subgraph "API Query Logic"
        F{"Query Time Range?"};
        F -- "< 24h" --> B;
        F -- "1-7d" --> C;
        F -- "7-30d" --> D;
        F -- "> 30d" --> E;
    end