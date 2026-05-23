# CoinBot-OpenCore

### The OpenCore Philosophy
To maintain the highest standards of security and data integrity, we have adopted an **OpenCore model**:

* **Transparency:** The core authentication flow, state management, and orchestration layers are fully open-source to allow for proof of action read access.
* **Transaction Integrity:** The transaction processing logic is open to the public, ensuring that every on-chain interaction is verifiable, secure, and follows strict EIP standards.
* **Extensibility:** While specific proprietary strategies remain private, this repository provides the "hooks" and infrastructure for developers to build, test, and integrate their own custom features into the Mint ecosystem — **contributions are welcome by making a pull request!**

---


## System Architecture

The system follows a classic client-server architecture optimized for Web3 security and low-latency execution, fully containerized for development and production parity.

### 1. Authentication Flow (EIP-191)

- **Challenge:** React frontend requests a unique nonce from the Rust server via `/api/user/auth`.
- **State:** Server generates a UUID nonce and stores it in a **Moka** cache (5-min TTL).
- **Signature:** User signs the nonce using **Ethers v6** via their browser wallet.
- **Verification:** Server recovers the address; if valid, it establishes a session.

#### Security Layers
- **Layer 1 ```Defualt```:** 5-minute session.
- **Layer 2 ```Optional```:** Layer 1 + IP binding.
  - Sessions are locked to one page at a time in **read-only mode** — no interaction with objects on the page.
  - If the IP has not changed, the session **automatically renews** for another 5 minutes on the same page.
  - To access a new page or perform an emergency action (withdraw, stop trade), a **new session must be obtained**.

#### 2. Core Components

- **State Manager:** Centralized `AppState` cloned across Actix workers (PostgreSQL pool, KuCoin client, Moka caches).
- **Transaction Layer:** Logic for validating and broadcasting on-chain interactions, ensuring auditability.
- **Trading Engine:** (Planned) Volatility monitoring and automated execution.
- **Orchestration:** **Docker Compose** manages networking and environment synchronization between the UI and API.

---

### File System Structure

```text
.
├── docker-compose.yml         # Orchestration for Frontend & Backend
├── server/                    # Rust Backend (Actix-Web)
│   ├── src/                   # Handlers, Models, Routes, State, Transaction Logic
│   ├── Cargo.toml             # Backend dependencies
│   └── Dockerfile             # Multi-stage Rust build
└── react/                     # Frontend (Vite + TS)
    ├── src/                   # Components, Hooks, Pages, Services
    ├── package.json           # Frontend dependencies
    └── Dockerfile             # Node.js development environment
```

---

### Tech Stack

| Layer | Technology |
| :--- | :--- |
| **Backend** | Rust (Actix-Web), PostgreSQL (SQLx), Moka Cache |
| **Frontend** | React 19, TypeScript, Vite, Ethers v6, Axios |
| **DevOps** | **Docker**, **Docker Compose** |
| **Web3** | EIP-191 Signature Verification (ethers-rs / ethers.js) |

---

### Development Setup

**Prerequisites:** Docker & Docker Compose installed.

1. **Configure Environment:**
   Ensure your `.env` file exists in the root with `DATABASE_URL` and your specific API credentials.

2. **Launch System:**

```bash
   docker compose up --build
   
```

   _Note for Fedora users: The volume mounts utilize the `:Z` flag to handle SELinux permissions._

3. **Access:**
   - **Frontend:** `http://localhost:5173`
   - **Backend API:** `http://localhost:8080`

---

### Contributing

We believe in open tools for a decentralized world. If you find a bug or have a feature request for the core infrastructure or transaction processing layer, let us know <3
