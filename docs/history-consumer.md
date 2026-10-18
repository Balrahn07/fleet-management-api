# Vehicle status history consumer

A separate Rust binary consumes vehicle-events and records valid status events
in vehicle_status_history. See the [README](../README.md) for environment setup
and creation of both Kafka topics.

```text
sqlx migrate run
cargo run --bin vehicle-history-consumer
```

Start the API in another terminal with cargo run. DATABASE_URL selects the
history database, KAFKA_BROKERS defaults to localhost:9092, and
KAFKA_HISTORY_GROUP defaults to vehicle-status-history.

## Save before acknowledging

Processing is sequential: receive, validate, INSERT history, then commit
offset+1. Automatic offset storage and commits are disabled. The synchronous
Kafka commit runs on a blocking thread, outside Tokio's async workers.

If the consumer exits after saving but before committing, Kafka can replay the
record. The event_id primary key and ON CONFLICT DO NOTHING make that replay
safe. This is at-least-once delivery with idempotent database writes, not a
transaction spanning Kafka and PostgreSQL.

History has no foreign key to vehicles: delayed events must remain recordable
after a vehicle is deleted. occurred_at is the event time; recorded_at is the
insertion time. Arrival order is not guaranteed to be chronological.

```sql
SELECT event_id, vehicle_id, status, occurred_at, recorded_at
FROM vehicle_status_history
ORDER BY occurred_at, event_id;
```

The same consumer group resumes from its committed offsets; a new group reads
retained records from the beginning. One partition permits one active consumer
per group. Event-ID deduplication still applies across groups.

## Failure policy

- Invalid JSON, unknown types, missing payloads, mismatched IDs, or invalid
  statuses are sent to vehicle-events-dlq. Only successful delivery allows the
  original offset to be committed. See [dead-letter handling](dead-letter.md).
- Transient database failures retry five times, waiting 1, 2, 4, then 8 seconds.
  Each write has a 10-second timeout, below the five-minute Kafka poll interval.
- Schema/constraint failures stop immediately. Exhausted database retries, DLQ
  delivery failures, receive errors, or offset-commit failures exit nonzero.
  Database failures never route otherwise valid messages to the DLQ.
- Ctrl+C stops consumption. An in-flight write or DLQ delivery may have completed
  without an offset commit; restart can therefore cause safe history replay or
  a duplicate DLQ record.

## Tests

```text
cargo test --locked
cargo test --locked --test history -- --ignored
```

The standard suite checks event validation, concurrent duplicate history writes,
multiple events per vehicle, database retries, and dead-letter byte preservation.
The ignored test uses real Kafka plus an isolated SQLx database. It simulates the
save-before-commit crash window, verifies failed DLQ delivery leaves the offset
unchanged, restarts and forwards successfully, processes a subsequent valid
event, and confirms a database failure neither commits nor dead-letters.
