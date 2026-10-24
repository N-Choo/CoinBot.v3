# gRPC in CoinBot.v3

## What is gRPC?

A way for processes to call functions on each other over the network — like a local
function call, but crossing process boundaries. Built on HTTP/2 + Protocol Buffers.

## The Contract: `.proto` files

Every gRPC service starts with a `.proto` file. It defines **messages** (data shapes)
and **services** (RPC endpoints). This is the single source of truth — both server
and client compile from the same file.

### Messages — "what data looks like"

```protobuf
message TicketRequest {
  string tx_hash = 1;
  string uid = 2;
  string ticker = 5;
}
message TicketResponse {
  string ticket_id = 1;
  string timestamp = 2;
  bool accepted = 3;
}
```

Rules:
- Every field has a **type** (`string`, `int32`, `bool`, etc.), a **name**, and a **field id** (integer)
- The field id is the **wire tag** — what gets sent over the network (not the name)
- Rename the field freely (`user_id` → `customer_id`), keep the same field id → **backward compatible**
- Change the field id → **breaks everything** (old clients still send old id)
- Field id range: **1 to 536,870,911** (2²⁹ - 1)
- **1–15** = 1 byte on wire (use for most common fields)
- **16–2047** = 2 bytes on wire
- **19000–19999** = **reserved by protobuf** — cannot use
- Removing a field? Comment it out, don't delete (reserve the id)

### Services — "what you can call"

```protobuf
service DepositService {
  rpc CreateTicket0(TicketRequest) returns (TicketResponse);
}
```

Compiled by `tonic-build` → generates:
- **Server trait** (`DepositService` + `DepositServiceServer` wrapper)
- **Client stub** (`DepositServiceClient`)

## Client vs Server

```
┌─ Server (deposit-worker) ────────────────┐
│                                           │
│  struct DepositServer;                    │
│                                           │
│  impl DepositService for DepositServer {   │
│    async fn create_ticket0(...) { ... }   │
│  }                                        │
│                                           │
│  Server::builder()                        │
│    .add_service(DepositServiceServer::new(...))
│    .serve("0.0.0.0:50051")               │
│    .await?                                │
└───────────────────────────────────────────┘

┌─ Client (api_gateway) ──────────────────┐
│                                          │
│  let mut client =                        │
│    DepositServiceClient::connect(        │
│      "http://deposit-worker:50051"       │
│    ).await?;                              │
│                                          │
│  let resp = client                       │
│    .create_ticket0(TicketRequest { ... })│
│    .await?;                               │
│                                          │
│  println!("{}", resp.ticket_id);         │
└──────────────────────────────────────────┘
```

### Why field id matters

```protobuf
message TicketRequest {
  string tx_hash = 1;   // id 1 → sent as 1 on the wire
}

// Safe evolution:
message TicketRequest {
  string transaction_hash = 1;  // renamed, id 1 still = same wire tag → old clients work
}

// Breaking change:
message TicketRequest {
  string tx_hash = 2;  // changed from 1 → 2, old clients send id 1 → server sees unknown field
}
```

## Proto Types → Rust Types

| Proto | Rust |
|---|---|
| `string` | `String` |
| `int32` | `i32` |
| `int64` | `i64` |
| `bool` | `bool` |
| `repeated T` | `Vec<T>` |
| `message Nested` | `struct Nested` |

## How `share` ties it together

```
proto/wallet.proto            ← single source of truth
        │
 share/build.rs               ← tonic-build compiles proto at compile time
        │
 share/src/lib.rs             ← re-exports generated types + traits
        │
        ├── deposit-worker    ← implements DepositService trait (server)
        └── api_gateway       ← calls via DepositServiceClient (client)

share/src/db/                 ← DB models, Ethereum RPC, ABI decoding
        │
        ├── deposit-worker    ← Deposit::create_pending, Rpc::get_transaction
        └── api_gateway       ← User::uid_for_wallet, Rpc::get_transaction
```

`share` holds the proto **contract** (types + traits) and the shared implementation
(DB queries, blockchain calls) in one crate. If the proto changes, `share` recompiles and
every consuming service must follow.

## Proto service organization

The proto currently defines a single service:

```protobuf
service DepositService {
  rpc CreateTicket0(TicketRequest) returns (TicketResponse);
}
```

- `DepositService` is **implemented** by `deposit-worker` (gRPC server)
- `api_gateway` can call it via `DepositServiceClient`

## Separation of Duties (SoD)

Each business function runs in an isolated process. No single service performs
both deposit and withdrawal logic.

| Process | Responsibility | Cannot do |
|---|---|---|
| `deposit-worker` | Record deposits via gRPC | Process withdrawals |
| `api_gateway` | Auth, routing | Modify balances |

Enforced by:

1. **Crate boundaries** — services depend only on the shared `share` crate, never on each other
2. **Schema ownership** — each service owns its own tables (`deposits` vs `users`/`contracts`)
3. **gRPC contracts** — services only expose what the proto defines

## Ethical Walls

An ethical wall prevents information flow between services that should not share data.

In this architecture, the wall is **inherent in the process boundaries**:

```
┌─ deposit-worker ─────────────┐
│                               │
│  owns: deposits DB            │
│  exposes: CreateTicket0       │  ← gRPC: "record this deposit"
│                               │
│  ❌ Cannot: auth sessions     │  ← no access to users table
│  ❌ Cannot: read api keys     │  ← not in schema
│                               │
└───────────────────────────────┘
```

`api_gateway` never touches deposit data directly. It calls `CreateTicket0`
through gRPC and gets a controlled response. The two services share infrastructure
(one PostgreSQL, one Redis) but not each other's code or tables.

**Three layers of enforcement:**

| Layer | Mechanism |
|---|---|
| Rust compiler | Services import only the shared `share` crate — no cross-service imports |
| Schema | Each service touches only its own tables |
| Proto | Services only expose what the `.proto` allows |

## Adding a new RPC

1. Add the message types and RPC to `proto/wallet.proto`
2. Rebuild — `tonic-build` regenerates stubs automatically
3. Implement the new trait method in the server crate
4. Call the new method from the client crate
5. If the server or client doesn't implement the new RPC, **compilation fails**

This is the guarantee — mismatched contracts don't compile.

## Running the services

```sh
# Terminal 1 — start deposit-worker gRPC server
cargo run -p deposit

# Terminal 2 — start api_gateway HTTP server
cargo run -p api-gateway
```

## Redis Communication

gRPC handles synchronous calls (request/response); Redis handles asynchronous messaging
and shared state:

### Pub/Sub — the signal pipeline

The engine and analyzer talk over two fire-and-forget channels:

```
trade-engine ──PUB "tickers:analyze"──▶ analyzer
analyzer     ──PUB "signals:result" ──▶ trade-engine
```

| Channel | Direction | Payload |
|---|---|---|
| `tickers:analyze` | engine → analyzer | `["BTC/USDT", …]` |
| `signals:result`  | analyzer → engine | `[{ ticker, action, confidence, entry_price, reason }]` |

If no consumer is listening the message is lost — acceptable for the periodic ticker
refresh and signal fan-out.

### Key/value — sessions and nonces

The `api_gateway` uses Redis (via `share::cache::Cache`) for the login **nonce** cache and
the **session** cache, each on its own Redis database.

## Further reading

- [tonic docs](https://docs.rs/tonic)
- [Protocol Buffers guide](https://protobuf.dev/programming-guides/proto3/)
