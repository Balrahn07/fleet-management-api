# Try the outbox with Kafka

This setup uses the official Apache Kafka image from the
[Apache quickstart](https://kafka.apache.org/quickstart/).
Run these commands from the project directory in PowerShell.
PostgreSQL and Redis must already be running and `.env` must contain their URLs.
The sample `.env.example` lists all required connection settings.

## Start Kafka and create the topic

```powershell
docker compose -f compose.kafka.yaml up -d --wait
docker compose -f compose.kafka.yaml exec -T kafka /opt/kafka/bin/kafka-topics.sh --bootstrap-server localhost:9092 --create --if-not-exists --topic vehicle-events --partitions 1 --replication-factor 1
docker compose -f compose.kafka.yaml exec -T kafka /opt/kafka/bin/kafka-topics.sh --bootstrap-server localhost:9092 --create --if-not-exists --topic vehicle-events-dlq --partitions 1 --replication-factor 1
cargo run --locked
```

The broker is exposed only on localhost:9092. This is a single-broker learning
setup, with one partition and one replica. It is not a production deployment.
Kafka data survives stop/start of this container. This setup has no persistent
volume: removing/recreating the container can lose its topic data.

## Watch events

In another terminal:

```powershell
docker compose -f compose.kafka.yaml exec -T kafka /opt/kafka/bin/kafka-console-consumer.sh --bootstrap-server localhost:9092 --topic vehicle-events --from-beginning --formatter-property print.key=true
```

The consumer prints the vehicle UUID as the key, then the JSON envelope.
`--from-beginning` replays existing records as well as watching new ones.
Ctrl+C stops this console consumer.

## Create and update a demo vehicle

In another PowerShell terminal:

```powershell
$vin = 'E2E' + [Guid]::NewGuid().ToString('N').Substring(0,14).ToUpperInvariant()
$body = @{ vin = $vin; model = 'Kafka integration demo' } | ConvertTo-Json
$vehicle = Invoke-RestMethod http://127.0.0.1:3000/vehicles -Method Post -ContentType 'application/json' -Body $body
Invoke-RestMethod "http://127.0.0.1:3000/vehicles/$($vehicle.id)" -Method Put -ContentType 'application/json' -Body '{"status":"online"}'
```

Creating a vehicle currently emits no event. Updating its status emits
VehicleStatusChanged, which should appear in the console consumer.

## Simulate an outage

```powershell
docker compose -f compose.kafka.yaml stop kafka
Invoke-RestMethod "http://127.0.0.1:3000/vehicles/$($vehicle.id)" -Method Put -ContentType 'application/json' -Body '{"status":"maintenance"}'
```

The request still succeeds. Inspect the following query in your PostgreSQL client
using the returned vehicle UUID:

```sql
SELECT id, payload, published_at
FROM outbox_events
WHERE aggregate_id = '<vehicle UUID>'
ORDER BY created_at;
```

The maintenance event has a NULL published_at while Kafka is down. Restore Kafka:

```powershell
docker compose -f compose.kafka.yaml up -d --wait
```

The worker retries automatically. The maintenance event appears in Kafka and
published_at becomes non-NULL. Restart the console consumer if it exited during
the outage. Duplicate delivery remains possible; consumers should deduplicate
using the envelope's event ID.

## Stop the demo

Use Ctrl+C in the API and consumer terminals. Stop Kafka with:

```powershell
docker compose -f compose.kafka.yaml stop kafka
```

## Verified locally

On 2026-09-12, the Windows API successfully delivered an online event to Kafka
4.3.1. With Kafka stopped, a maintenance update succeeded and its event remained
pending in PostgreSQL. Restarting Kafka delivered the same pending event, and
both rows had published_at set. The demo vehicle was kept for inspection:
`ae592235-6999-4098-a36b-f1c38e4dc8e9` (model: Kafka integration demo).
