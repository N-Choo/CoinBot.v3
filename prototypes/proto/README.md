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
message BalanceRequest {
  string user_id = 1;   // field name + field id
}
message BalanceResponse {
  string balance = 1;   // string is fine for money (no float rounding)
  string currency = 2;
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
  rpc GetBalance(BalanceRequest) returns (BalanceResponse);
  // ↑ service name    ↑ RPC name  ↑ input      ↑ output
}
```

Compiled by `tonic-build` → generates:
- **Server trait** (`DepositService` + `DepositServiceServer` wrapper)
- **Client stub** (`DepositServiceClient`)

## Client vs Server

```
┌─ Server (deposit) ───────────────────┐
│                                       │
│  struct DepositServer;                 │
│                                        │
│  impl DepositService for DepositServer {  ← implement the trait
│    async fn get_balance(...) { ... }   │
│  }                                     │
│                                        │
│  Server::builder()                     │
│    .add_service(DepositServiceServer::new(...))  ← register
│    .serve("0.0.0.0:50051")            ← listen on port
│    .await?                             │
└────────────────────────────────────────┘

┌─ Client (withdrawal) ─────────────────┐
│                                        │
│  let mut client =                      │
│    DepositServiceClient::connect(      │
│      "http://deposit:50051"            │  ← connect to server
│    ).await?;                            │
│                                        │
│  let resp = client                     │
│    .get_balance(BalanceRequest {       │  ← call like a function
│      user_id: "abc".into()             │
│    }).await?;                           │
│                                        │
│  println!("{}", resp.balance);         │  ← use the response
└────────────────────────────────────────┘
```

### Why field id matters

```protobuf
message BalanceRequest {
  string user_id = 1;   // id 1 → sent as 1 on the wire
}

// Safe evolution:
message BalanceRequest {
  string customer_id = 1;  // renamed, id 1 still = same wire tag → old clients work
}

// Breaking change:
message BalanceRequest {
  string user_id = 2;  // changed from 1 → 2, old clients send id 1 → server sees unknown field
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

## How `common` ties it together

```
proto/wallet.proto          ← one file, one truth
        │
 common/build.rs            ← runs at compile time, reads proto, generates Rust
        │
 common/src/lib.rs          ← re-exports generated stubs
       ╱    ╲
deposit    withdrawal       ← both get identical types
```

The `common` crate is only a library — it has no `main.rs` and cannot run.
It exists purely so deposit and withdrawal share the **exact same** message types
and service traits. If the proto changes, both must recompile.

## Proto service organization

Each business domain gets its own service in the proto:

```protobuf
service DepositService {
  rpc GetBalance(BalanceRequest) returns (BalanceResponse);
  rpc GetHistory(HistoryRequest) returns (HistoryResponse);
}

service WithdrawalService {
  rpc SubmitWithdrawal(WithdrawalRequest) returns (WithdrawalResponse);
}

service GatewayService {
  rpc GetDashboard(DashboardRequest) returns (DashboardResponse);
}
```

- DepositService is **implemented** by the deposit crate (server)
- WithdrawalService is **implemented** by the withdrawal crate (server) or called via client
- GatewayService would be an aggregation endpoint in api_gateway

## Separation of Duties (SoD)

Each business function runs in an isolated process. No single service performs
both deposit and withdrawal logic.

| Process | Responsibility | Cannot do |
|---|---|---|
| `deposit` | Record deposits, report balance | Process withdrawals |
| `withdrawal` | Process withdrawals | Access deposit DB |
| `api_gateway` | Auth, routing, UI push | Modify balances |
| `trade-engine` | ML signals | Access any wallet data |

Enforced by:

1. **Crate boundaries** — no crate imports another service's internals
2. **Database isolation** — each service has its own DB credentials
3. **gRPC contracts** — services only expose what the proto defines

## Ethical Walls

An ethical wall prevents information flow between services that should not share data.

In this architecture, the wall is **inherent in the process boundaries**:

```
┌─ Withdrawal ─────────────────┐
│                               │
│  wants: user balance          │
│  has access to: withdrawal DB │
│                               │
│  ✅ Can call: GetBalance      │  ← gRPC: "does user have 500?"
│  ❌ Cannot: query deposit DB  │  ← no credentials
│  ❌ Cannot: read deposit rows │  ← not in schema
│                               │
└───────────────────────────────┘
```

The withdrawal service never touches deposit data directly. It asks the deposit
service "does this user have enough funds?" through gRPC and gets a controlled
answer. No shared database, no shared memory, no shared secrets.

**Three layers of enforcement:**

| Layer | Mechanism |
|---|---|
| Rust compiler | Crates only import `common` — no cross-service imports |
| Network | Docker — each container isolated, no DB cross-connect |
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
# Terminal 1 — start deposit server
cargo run -p deposit

# Terminal 2 — call it from withdrawal client
cargo run -p withdrawal
```

## Redis Communication

gRPC handles sync calls (request-response). Redis handles async messages (events, signals).
Two Redis patterns are used:

### Redis Streams — persistent, replayable messages

Best for: transaction events, trade signals (must not lose data).

```
deposit ──▶ "tx:deposits"     stream ──▶ api_gateway consumes → push to UI
withdrawal ──▶ "tx:withdrawals" stream ──▶ api_gateway consumes → push to UI
trade-engine ──▶ "trade:signals" stream ──▶ deposit/withdrawal consume → act on signal
```

Streams persist until explicitly trimmed. If a consumer disconnects, it can resume
from where it left off — unlike pub/sub.

**Message format** (JSON — easy cross-language):

```json
{
  "type": "deposit",
  "user_id": "user_abc",
  "amount": "500.00",
  "currency": "USDT",
  "tx_id": "dep-tx-001",
  "timestamp": 1718000000
}
```

### Redis Pub/Sub — fire-and-forget

Best for: live dashboard updates, status notifications (ok to lose).

```
deposit ──▶ "status:updates" pub ──▶ api_gateway → WebSocket → React
```

If no one is listening, the message disappears. That's fine for live updates —
the next one will arrive.

### Key naming conventions

| Pattern | Example |
|---|---|
| `tx:<type>` | `tx:deposits`, `tx:withdrawals` |
| `trade:<topic>` | `trade:signals` |
| `status:<service>` | `status:deposit`, `status:withdrawal` |

All services share the same Redis instance but use different key prefixes.
Enforcement: a service only has permission for its own key prefix (Docker network policy).

## Further reading

- [tonic docs](https://docs.rs/tonic)
- [Protocol Buffers guide](https://protobuf.dev/programming-guides/proto3/)
