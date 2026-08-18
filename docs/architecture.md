# Fleet Management API --- Project Notes

## Goal

This project is a Rust backend API for managing vehicles in a fleet
management platform.

It teaches backend fundamentals using:

-   Rust
-   Axum
-   PostgreSQL
-   SQLx
-   REST APIs
-   Layered backend architecture

------------------------------------------------------------------------

## Current Architecture

Request flow:

``` text
HTTP Request
    ↓
Router
    ↓
Handler
    ↓
Service
    ├── Cache (Redis via Cache trait)
    ↓
Repository
    ↓
PostgreSQL
```

For cached reads such as `GET /vehicles/{id}`, the service uses the
cache-aside pattern:

``` text
Request → Redis
            ├── HIT  → deserialize → return Vehicle
            └── MISS → PostgreSQL → serialize/store in Redis → return Vehicle
```

Each layer has a separate responsibility.

------------------------------------------------------------------------

## `main.rs`

Application entry point.

Responsibilities:

-   Load environment variables from `.env`
-   Initialize logging
-   Read `DATABASE_URL`
-   Create the PostgreSQL connection pool
-   Build `AppState`
-   Create routes
-   Start the Axum HTTP server

Important concept:

``` rust
PgPool
```

is a PostgreSQL connection pool shared by the application.

------------------------------------------------------------------------

## `routes.rs`

Defines API routes.

Example:

``` rust
.route("/vehicles", get(list_vehicles))
.route("/vehicles", post(create_vehicle))
.route("/vehicles/{id}", get(get_vehicle))
```

Responsibilities:

-   Map HTTP method + path to handler function
-   Attach shared application state using `.with_state(state)`

Examples:

``` text
GET    /vehicles      → list_vehicles
POST   /vehicles      → create_vehicle
GET    /vehicles/{id} → get_vehicle
```

------------------------------------------------------------------------

## `handlers.rs`

HTTP layer.

Responsibilities:

-   Extract data from the HTTP request
-   Call the service layer
-   Return HTTP responses

Examples of Axum extractors:

``` rust
State(state): State<AppState>
Path(id): Path<Uuid>
Json(request): Json<CreateVehicleRequest>
```

Meaning:

-   `State<AppState>` gets shared application state
-   `Path<Uuid>` gets the ID from the URL
-   `Json<T>` gets JSON body data

Handlers should not contain business logic.

------------------------------------------------------------------------

## `services.rs`

Business logic layer.

Responsibilities:

-   Validate input
-   Apply business rules
-   Call repositories
-   Convert repository errors into HTTP-level errors for now

Example business rules:

-   VIN cannot be empty
-   Model cannot be empty
-   New vehicles start with status `"offline"`
-   Missing vehicle becomes `404 Not Found`
-   Database error becomes `500 Internal Server Error`

Example:

``` rust
Result<Vehicle, StatusCode>
```

means:

-   `Ok(vehicle)` → success
-   `Err(StatusCode)` → HTTP error response

------------------------------------------------------------------------

## `repositories.rs`

Database access layer.

Responsibilities:

-   Execute SQL queries using SQLx
-   Read from PostgreSQL
-   Insert into PostgreSQL
-   Return database results to the service layer

Example:

``` rust
sqlx::query_as!(
    Vehicle,
    "SELECT id, vin, model, status, created_at, updated_at FROM vehicles"
)
```

The repository should not know about HTTP status codes.

It returns database errors such as:

``` rust
sqlx::Error
```

------------------------------------------------------------------------

## `models.rs`

Data structures.

Current models:

``` rust
Vehicle
CreateVehicleRequest
```

### `Vehicle`

Represents a vehicle stored in the system.

Contains backend-managed fields:

-   `id`
-   `status`
-   `created_at`
-   `updated_at`

### `CreateVehicleRequest`

Represents JSON sent by the client when creating a vehicle.

Contains only client-provided fields:

-   `vin`
-   `model`

Important concept:

``` text
Vehicle != CreateVehicleRequest
```

DTOs protect the backend from letting clients control fields like `id`
or `status`.

------------------------------------------------------------------------

## `state.rs`

Shared application state.

Currently contains shared dependencies similar to:

``` rust
pub struct AppState {
    pub db: PgPool,
    pub cache: Arc<dyn Cache>,
}
```

This is dependency injection: the application creates shared
dependencies once and injects them through `AppState`.

`Arc<dyn Cache>` lets concurrent requests share the same cache while the
service depends on the `Cache` abstraction rather than directly on
`RedisCache` or `InMemoryCache`.

Before PostgreSQL, this project used:

``` rust
Arc<Mutex<Vec<Vehicle>>>
```

That was temporary in-memory state.

Now PostgreSQL stores data persistently.

------------------------------------------------------------------------

## `migrations/`

Contains SQL migration files.

Migrations describe database schema changes.

Example:

``` sql
CREATE TABLE vehicles (
    id UUID PRIMARY KEY,
    vin TEXT NOT NULL UNIQUE,
    model TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

SQLx tracks applied migrations in:

``` text
_sqlx_migrations
```

------------------------------------------------------------------------

## Database Table: `vehicles`

Columns:

``` text
id          UUID PRIMARY KEY
vin         TEXT NOT NULL UNIQUE
model       TEXT NOT NULL
status      TEXT NOT NULL
created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
```

Important constraints:

-   `id` is the primary key
-   `vin` must be unique
-   `vin`, `model`, and `status` cannot be null

------------------------------------------------------------------------

## Important Rust Concepts Used

### `State<AppState>`

Axum extractor meaning:

``` text
Get AppState from application state.
```

### `Path<Uuid>`

Axum extractor meaning:

``` text
Get UUID from URL path.
```

### `Json<T>`

Axum extractor meaning:

``` text
Deserialize request/response JSON as T.
```

### `.await`

Waits for an async operation to finish.

Used for database calls.

### `map_err`

Converts one error type into another.

Example:

``` rust
.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
```

converts:

``` rust
sqlx::Error
```

into:

``` rust
StatusCode
```

### `ok_or`

Converts:

``` rust
Option<T>
```

into:

``` rust
Result<T, E>
```

Example:

``` rust
option.ok_or(StatusCode::NOT_FOUND)
```

means:

-   `Some(vehicle)` → `Ok(vehicle)`
-   `None` → `Err(404)`

------------------------------------------------------------------------

## Current Endpoints

### Health check

``` http
GET /
```

Returns:

``` text
OK
```

### List vehicles

``` http
GET /vehicles
```

Returns all vehicles from PostgreSQL.

### Get vehicle by ID

``` http
GET /vehicles/{id}
```

Returns:

-   `200 OK` with vehicle JSON if found
-   `404 Not Found` if missing

### Create vehicle

``` http
POST /vehicles
```

Request body:

``` json
{
  "vin": "VF123",
  "model": "Tesla Model Y"
}
```

Backend generates:

-   `id`
-   `status = "offline"`
-   `created_at`
-   `updated_at`

------------------------------------------------------------------------

### Update vehicle status

``` http
PUT /vehicles/{id}
```

Request body:

``` json
{
  "status": "maintenance"
}
```

Returns:

-   200 OK with updated vehicle JSON if found
-   400 Bad Request if status is invalid
-   404 Not Found if missing

### Delete vehicle

``` http
DELETE /vehicles/{id}
```

Returns:

-   204 No Content if deleted
-   404 Not Found if missing

------------------------------------------------------------------------

# Caching

## Why Cache?

Without caching, repeated reads always reach PostgreSQL:

``` text
Request → PostgreSQL → Response
```

With caching:

``` text
Request → Cache
            ├── HIT  → Response
            └── MISS → PostgreSQL → Cache → Response
```

This reduces database load and can reduce response latency.

## Cache Abstraction

The application uses a trait so business logic does not depend directly
on a particular cache implementation:

``` rust
#[async_trait]
pub trait Cache: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<String>, CacheError>;
    async fn set(&self, key: &str, value: String) -> Result<(), CacheError>;
    async fn remove(&self, key: &str) -> Result<(), CacheError>;
}
```

Implementations can include:

``` text
Cache
├── InMemoryCache
└── RedisCache
```

Important concepts:

-   `async_trait` allows the async cache interface to be conveniently
    used through `dyn Cache`.
-   `Send` means an implementation can safely move across threads.
-   `Sync` means it can safely be shared between threads.
-   `Arc` provides shared ownership of the cache between concurrent
    requests.

## Cache-Aside Pattern

For:

``` http
GET /vehicles/{id}
```

the service:

1.  Builds a key such as `vehicle:{id}`.
2.  Checks Redis.
3.  On a cache hit, deserializes the JSON into `Vehicle`.
4.  On a cache miss, queries PostgreSQL.
5.  Serializes the returned `Vehicle` to JSON.
6.  Stores it in Redis.
7.  Returns the vehicle.

Serialization:

``` text
Vehicle → JSON String
```

Deserialization:

``` text
JSON String → Vehicle
```

## Cache Errors

The cache interface returns:

``` rust
Result<Option<String>, CacheError>
```

This distinguishes:

``` text
Ok(Some(value)) → cache hit
Ok(None)        → cache miss
Err(error)      → cache/backend failure
```

A Redis failure should generally not make a successful read from
PostgreSQL fail. The service can log the cache failure and fall back to
PostgreSQL.

PostgreSQL remains the source of truth.

## Cache Invalidation

After a successful update or delete, remove the corresponding cache
entry:

``` text
Database modification succeeds
        ↓
remove vehicle:{id} from cache
```

Invalidation should happen after the database operation succeeds.

If invalidation fails, the application can log a warning and continue,
although stale data may remain until expiration.

## TTL

TTL means **Time To Live**.

A cached value is valid only for a configured period, for example 60
seconds.

With Redis, TTL can be set when storing the value:

``` text
SETEX key 60 value
```

After expiration, the next read becomes a cache miss and reloads the
value from PostgreSQL.

## In-Memory Cache vs Redis

An in-memory cache belongs to one API process:

``` text
API A → Cache A
API B → Cache B
API C → Cache C
```

Redis provides a shared cache:

``` text
API A ─┐
API B ─┼→ Redis
API C ─┘
```

This is useful when multiple instances of the backend are running.

## Cache Stampede

A cache stampede can happen when a hot key expires and many clients
request it simultaneously:

``` text
vehicle:123 expires
        ↓
10,000 cache misses
        ↓
10,000 requests may reach PostgreSQL
```

This can consume database CPU and connection-pool capacity and increase
latency.

### Request Coalescing / Single-Flight

For one hot key, concurrent identical work can be combined:

``` text
Request A → MISS → acquire per-key lock → PostgreSQL → fill cache
Request B → MISS → wait
Request C → MISS → wait
```

After A fills the cache, waiting requests check the cache again and
reuse the result.

For one process, a possible Rust building block is:

``` rust
DashMap<String, Arc<tokio::sync::Mutex<()>>>
```

For multiple API instances, local locks are not enough; distributed
coordination may be needed.

### TTL Jitter

TTL jitter addresses a different problem: **many different keys expiring
together**.

Instead of every key having exactly 60 seconds:

``` text
vehicle:1 → 60s
vehicle:2 → 60s
vehicle:3 → 60s
```

add a small random variation:

``` text
vehicle:1 → 54s
vehicle:2 → 67s
vehicle:3 → 61s
```

This spreads expiration-related database load over time.

Remember:

``` text
One hot key expires
→ request coalescing / single-flight

Many different keys expire together
→ TTL jitter
```

------------------------------------------------------------------------

# Message Brokers and Event-Driven Systems

Message brokers decouple synchronous HTTP work from asynchronous
processing.

Without a broker:

``` text
HTTP Request
→ API
→ Database
→ downstream work
→ Response
```

With a broker:

``` text
HTTP Request
→ API
→ Database
→ publish event
→ Response

             Broker
            ↙   ↓   ↘
       Analytics Audit Alerts
```

## Producer and Consumer

A **producer** publishes a message or event.

A **consumer** receives and processes it.

For example, the Fleet API could produce:

``` json
{
  "event": "vehicle_status_changed",
  "vehicle_id": "...",
  "old_status": "offline",
  "new_status": "online"
}
```

Analytics, audit, and alerting services could consume that event
independently.

## RabbitMQ vs Kafka --- Mental Model

Simplified mental model:

``` text
RabbitMQ → "Do this."
Kafka    → "This happened."
```

RabbitMQ is commonly used as a message/task broker where work is placed
on queues and acknowledged by consumers.

Kafka is an event-streaming platform built around retained, append-only
event logs that can be consumed and replayed independently.

## Acknowledgment and At-Least-Once Delivery

A consumer normally acknowledges successful processing.

A failure can occur after processing but before the acknowledgment is
recorded:

``` text
process message successfully
        ↓
consumer crashes before ACK
        ↓
message may be delivered again
```

This is why consumers often need to tolerate duplicate delivery.

## Idempotency

An idempotent operation has the same intended result if executed
multiple times.

Example:

``` sql
UPDATE vehicles
SET status = 'online'
WHERE id = '123';
```

Executing this repeatedly still leaves the status as `online`.

By contrast:

``` sql
UPDATE accounts
SET balance = balance + 100;
```

executed twice changes the balance twice.

Events can carry unique IDs so consumers can detect events they already
processed.

------------------------------------------------------------------------

# Kafka Fundamentals

## Topic

A Kafka **topic** is a named stream of events.

Example:

``` text
vehicle-events
```

It could contain:

``` text
VehicleCreated
VehicleStatusChanged
DriverAssigned
VehicleDeleted
```

Kafka normally retains events according to its retention configuration,
allowing consumers to read or replay them later.

## Partitions

A topic is divided into partitions:

``` text
vehicle-events

Partition 0: A → D → G
Partition 1: B → E → H
Partition 2: C → F → I
```

Partitions provide scalability and parallelism.

Different consumers can process different partitions concurrently.

## Ordering and Message Keys

Kafka guarantees ordering **within a partition**, not globally across
all partitions.

If ordering is required for all events belonging to one vehicle, use the
vehicle ID as the message key:

``` text
key = vehicle_id
```

Events with the same key are routed consistently to the same partition,
subject to the producer's partitioning strategy.

Therefore:

``` text
vehicle 123 events → same partition → ordered
vehicle 456 events → another partition → can run in parallel
```

Important design principle:

> Choose the partition key according to where ordering is required.

## Offset

Each event in a partition has a sequential **offset**:

``` text
Partition 0

offset 0 → VehicleCreated
offset 1 → StatusChanged
offset 2 → DriverAssigned
offset 3 → StatusChanged
```

Offsets let consumers track their progress through a partition.

They also make replay possible: a consumer can start again from an
earlier offset and reprocess historical events.

## Consumer Groups

Consumers working together use a **consumer group**.

Example:

``` text
group: analytics

Partition 0 → Analytics A
Partition 1 → Analytics B
Partition 2 → Analytics C
```

Within one consumer group, each partition is assigned to at most one
consumer at a time.

Consumers in the **same group** divide the work.

Different consumer groups independently consume the stream:

``` text
                 vehicle-events
                      │
          ┌───────────┼───────────┐
          ↓           ↓           ↓
      analytics     audit       alerts
        group        group        group
```

So:

``` text
Same consumer group      → split the work
Different consumer groups → each group receives the stream independently
```

## Partitions Limit Consumer Parallelism

If a topic has 3 partitions and a consumer group has 5 consumers:

``` text
Partition 0 → Consumer 1
Partition 1 → Consumer 2
Partition 2 → Consumer 3
Consumer 4  → idle
Consumer 5  → idle
```

Only three consumers can actively consume because there are only three
partitions.

## Kafka Mental Model

``` text
Topic
  ↓
Partitions
  ↓
Ordered events inside each partition
  ↓
Each event has an offset

Producer:
message key influences partition choice

Consumer group:
partitions are distributed among consumers
```

------------------------------------------------------------------------

