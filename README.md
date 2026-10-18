# Rust Fleet Management API

A learning project for backend engineering with Rust, Axum, PostgreSQL/SQLx,
Redis, and Kafka.

## What it does

- Vehicle CRUD, pagination, filtering, sorting, and driver assignment.
- Redis caching with database fallback.
- Transactional outbox: vehicle status updates and events commit together.
- A separate Kafka consumer builds an idempotent vehicle status history.
- Invalid Kafka messages go to a dead-letter topic; valid messages can continue.

```text
HTTP API -> PostgreSQL vehicle + outbox
                    |
              outbox publisher
                    |
             vehicle-events
                    |
             history consumer
               /          \
           valid          invalid
             |               |
     status history   vehicle-events-dlq
```

## Local setup

Prerequisites: Rust, Docker Desktop, PostgreSQL, Redis, and the SQLx CLI.
The Kafka dependency builds native code; install C/C++ build tools and CMake
for your platform. PostgreSQL must be running during compilation because
SQLx checks SQL queries at compile time.

1. Copy `.env.example` to `.env` if you don't already have one. Set DATABASE_URL
   to your development database and configure Redis/Kafka addresses.
2. Start PostgreSQL and Redis. If using the existing project containers:
   `docker start fleet-postgres fleet-redis`.
3. Apply migrations: `sqlx migrate run`.
4. Start Kafka and create both topics:

```text
docker compose -f compose.kafka.yaml up -d --wait
docker compose -f compose.kafka.yaml exec -T kafka /opt/kafka/bin/kafka-topics.sh --bootstrap-server localhost:9092 --create --if-not-exists --topic vehicle-events --partitions 1 --replication-factor 1
docker compose -f compose.kafka.yaml exec -T kafka /opt/kafka/bin/kafka-topics.sh --bootstrap-server localhost:9092 --create --if-not-exists --topic vehicle-events-dlq --partitions 1 --replication-factor 1
```

The Compose file starts Kafka only. Its single broker is a local learning setup.
Container removal/recreation can lose Kafka data because no persistent volume
is configured.

Run these in separate terminals:

```text
cargo run
cargo run --bin vehicle-history-consumer
```

The API listens on http://127.0.0.1:3000; its health endpoint is /health.
The consumer group defaults to vehicle-status-history, configurable through
KAFKA_HISTORY_GROUP. Ctrl+C stops each application.

## Testing

```text
cargo test --locked
cargo test --locked --test history -- --ignored
```

Standard tests require PostgreSQL. SQLx history/outbox tests create isolated
databases and need a role with database-creation privileges. API tests use
`.env.test` and truncate their tables: configure it to a dedicated test database.
The ignored history test additionally requires Kafka; it uses unique topics,
checks replay and dead-letter failure handling, and deletes its topics on success.

## Learn the design

- [Architecture and project notes](docs/architecture.md)
- [Outbox and Kafka demo](docs/kafka-demo.md)
- [Status history consumer](docs/history-consumer.md)
- [Dead-letter handling and inspection](docs/dead-letter.md)

Delivery is at least once. History is deduplicated by event ID. Dead-letter
records can also repeat and carry a stable source-position ID. There is no
automatic dead-letter replay, history HTTP endpoint, or production deployment
configuration yet.
