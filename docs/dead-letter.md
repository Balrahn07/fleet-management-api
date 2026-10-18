# Dead-letter handling

A dead-letter topic is a place to keep messages that cannot be interpreted by
this consumer. It prevents one malformed message from blocking all later work,
while preserving the rejected message for investigation.

## Routing and acknowledgement

```text
Receive from vehicle-events
  |
  +-- Valid -> save history -> commit original offset
  |
  +-- Invalid -> publish to vehicle-events-dlq
                   |
                   +-- Kafka confirms delivery -> commit original offset
                   +-- Delivery fails -> exit without committing
```

Database outages are not invalid messages: valid events use the existing retry
policy and remain unacknowledged if saving fails.

The consumer never automatically skips an event when DLQ forwarding fails.
A crash after forwarding but before committing can produce duplicate DLQ
records. The stable ID and Kafka key are source-topic:partition:offset.
Deduplicate on that ID when investigating or building replay tooling. This
identity is scoped to the source cluster and topic lifetime; it is not globally
unique if a topic is deleted/recreated or records are combined across clusters.

## What's preserved

The JSON envelope contains:

- id and original topic, partition, and offset.
- Original Kafka timestamp, when available.
- Original key and payload as optional arrays of byte values (0-255).
- Original headers, including binary values.
- Validation error and rejection timestamp.

Byte arrays preserve invalid UTF-8 without replacement characters. A null
payload means missing; [] means an empty payload. Error text is diagnostic data,
not an instruction. Forwarding keeps the original contents; treat this topic
with the same access and retention controls as the source.

Large byte-array envelopes can exceed Kafka's message-size limit. If Kafka
rejects the envelope, forwarding fails and the original offset is not committed.
Adjust limits or implement a separate large-message storage strategy before
using this with large production payloads.

## Create and inspect

```text
docker compose -f compose.kafka.yaml exec -T kafka /opt/kafka/bin/kafka-topics.sh --bootstrap-server localhost:9092 --create --if-not-exists --topic vehicle-events-dlq --partitions 1 --replication-factor 1
docker compose -f compose.kafka.yaml exec -T kafka /opt/kafka/bin/kafka-console-consumer.sh --bootstrap-server localhost:9092 --topic vehicle-events-dlq --from-beginning --formatter-property print.key=true
```

The consumer above prints the stable key followed by the JSON envelope.
Stop it with Ctrl+C. To demonstrate an invalid event, run this in PowerShell
while the history consumer is running:

```powershell
'not-json' | docker compose -f compose.kafka.yaml exec -T kafka /opt/kafka/bin/kafka-console-producer.sh --bootstrap-server localhost:9092 --topic vehicle-events
```

It should appear in the DLQ. A subsequent valid vehicle update should still
appear in status history. The deliberate demo record remains in the Kafka logs.

## Investigating a rejection

1. Inspect the error and source position.
2. Decode payload_bytes as UTF-8 only if those bytes form valid UTF-8.
3. Correct the producing application or update the consumer if it needs to
   support a legitimate new event type.
4. Decide whether and how to replay the corrected event.

There is no automatic replay or deletion in this implementation. Do not publish
the DLQ envelope itself to vehicle-events: that topic expects VehicleStatusEvent.
An unsupported type may be valid for another consumer; here it is quarantined
for review rather than silently ignored. Kafka retention still applies, so the
DLQ is not permanent archival storage.
