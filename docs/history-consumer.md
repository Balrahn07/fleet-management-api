# Vehicle status history consumer

The API and publisher keep their existing roles. A separate Rust binary consumes
`vehicle-events` and records each status event in `vehicle_status_history`.

## Run locally

Start PostgreSQL, Redis, and Kafka as described in [the Kafka demo](kafka-demo.md).
Apply migrations before starting the consumer (SQLx CLI required):

```text
sqlx migrate run
cargo run --bin vehicle-history-consumer
```

In another terminal, start the API with `cargo run` (the default binary is still
fleet-management-api). Follow the demo to update a vehicle.

Configuration:
- DATABASE_URL: PostgreSQL connection used for history.
- KAFKA_BROKERS: defaults to localhost:9092.
- KAFKA_HISTORY_GROUP: defaults to vehicle-status-history.

The same consumer group resumes from its committed offsets. A new group reads
retained records from the beginning. The history table still deduplicates by event
ID even when you use a different group. With one topic partition, only one member
of a group can actively consume that partition.

Inspect saved history in PostgreSQL:

```sql
SELECT event_id, vehicle_id, status, occurred_at, recorded_at
FROM vehicle_status_history
ORDER BY occurred_at, event_id;
```

## Why save before committing?

Processing is sequential: receive, validate, INSERT history, then commit offset+1.
Both automatic offset storage and automatic commits are disabled. The Kafka
commit is synchronous and runs on a blocking thread so it does not block Tokio's
async worker threads.

A crash after the INSERT but before the offset commit causes replay.
The primary key is event_id; INSERT ... ON CONFLICT DO NOTHING makes that replay
a no-op. A duplicate is acknowledged only after the database confirms success.
This is at-least-once delivery with idempotent database writes, not an atomic
transaction across Kafka and PostgreSQL.

The history has no foreign key to vehicles. A delayed event must still be saved
after its vehicle is deleted. It records event time separately from insertion
time. It is a history log, not an assertion that arrival order is chronological.

## Failures and shutdown

Transient database errors retry up to five times, with delays of 1, 2, 4, and 8
seconds. Each write has a 10-second timeout so processing remains below the
five-minute Kafka poll interval. Schema and constraint errors stop immediately.
If retries are exhausted or the Kafka offset commit fails, the process exits
nonzero and leaves the record available for replay when restarted.

Malformed JSON, unknown event types, missing payloads, mismatched vehicle IDs,
and invalid statuses stop the consumer without acknowledging the failing record.
The log identifies topic, partition, and offset. Investigate/fix the source or
consumer before restarting; repeatedly restarting cannot fix a poison message.
There is no dead-letter topic or automatic skip in this version.

Ctrl+C stops consumption. An in-flight write may already have succeeded, so a
subsequent start may replay it safely. A rebalance or ambiguous network failure
can also lead to duplicates, handled by the same event-ID constraint.

## Tests

```text
cargo test --locked
cargo test --locked --test history -- --ignored
```

The standard suite uses SQLx-managed isolated PostgreSQL databases. It checks
wire compatibility, invalid events, concurrent duplicate delivery, independent
events for one vehicle, and permanent database failures.

The ignored test also requires Kafka. It creates a unique topic and group,
saves an event without committing (simulating the crash window), restarts the
consumer, checks replay produces one history row, verifies the committed offset,
and verifies a malformed event leaves that offset unchanged. It removes its
topic after success. A failed test may leave its uniquely named history-test-*
topic for diagnosis.

Prerequisite: DATABASE_URL must connect to a PostgreSQL role allowed to create
test databases. The Kafka test uses KAFKA_BROKERS or localhost:9092.
