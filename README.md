## System Architecture

The system follows a classic client-server architecture optimized for Web3 security and low-latency execution.

#### 1. Authentication Flow (EIP-191)

- **Challenge Generation:** The React frontend requests a unique nonce from the Rust server via `/api/user/auth`.
- **Stateful Caching:** The server generates a UUID-based nonce and stores it in a **Moka** cache (5-minute TTL) mapped to the user's wallet address.
- **Cryptographic Signing:** The user signs the nonce using **Ethers v6** via their browser wallet (MetaMask/Rabby).
- **Verification:** The server recovers the public address from the signature. If the address matches and the nonce is valid, it invalidates the used nonce and establishes a session.
- **Session Management:** A session token is stored in a separate **Moka** cache (1-hour TTL) and delivered to the client via an HttpOnly cookie or JSON response.

#### 2. Core Components

- **State Manager:** A centralized `AppState` struct cloned across Actix workers, containing the **PostgreSQL** pool, **KuCoin** client, and asynchronous caches.
- **Trading Engine:** (Planned) Logic for monitoring price volatility and executing trades based on user-defined parameters.
- **Middleware:** The server utilizes **CORS**, **Logger**, and **NormalizePath** to ensure clean and secure API communication.

---

### File System Structure

```text
.
├── server/                      # Rust Backend (Actix-Web)
│   ├── src/
│   │   ├── handlers/            # Request logic
│   │   │   └── user/
│   │   │       ├── auth.rs      # Challenge & Login logic
│   │   │       └── tests/       # Unit & Integration tests
│   │   ├── models/              # DTOs and Data Structures
│   │   │   ├── auth.rs          # Auth Request/Response models
│   │   │   └── err.rs           # Centralized Error Handling
│   │   ├── routes.rs            # Scoped API routing definitions
│   │   ├── state.rs             # AppState (DB, Cache, KuCoin Client)
│   │   └── main.rs              # Server entry & Middleware setup
│   ├── Cargo.toml               # Backend dependencies
│   └── .env                     # Secrets (DB_URL, KuCoin Keys)
│
└── react/                       # Frontend (Vite + TS)
    ├── src/
    │   ├── components/          # Reusable UI (Topbar, Background)
    │   ├── hooks/               # useAuth, useTheme, useBot logic
    │   ├── pages/               # TradingPage, HomePage
    │   ├── services/            # connectWallet.tsx (Ethers logic)
    │   ├── styles/              # Glassmorphism & Global CSS
    │   └── App.tsx              # Router & Global Providers
    ├── tsconfig.json            # TypeScript configuration
    └── package.json             # Frontend dependencies
```

---

### Tech Stack

| Layer        | Technology                                             |
| :----------- | :----------------------------------------------------- |
| **Backend**  | Rust (Actix-Web), PostgreSQL (SQLx), Moka Cache        |
| **Frontend** | React 19, TypeScript, Vite, Ethers v6, Axios           |
| **Styling**  | Custom Glassmorphism CSS, Lucide-React Icons           |
| **Exchange** | KuCoin SDK Client                                      |
| **Web3**     | EIP-191 Signature Verification (ethers-rs / ethers.js) |

---

### Development Setup

1.  **Environment Variables:** Copy `.env.example` to `.env` in the `server/` directory and provide your PostgreSQL and KuCoin credentials.
2.  **Database:** Ensure PostgreSQL is running and run migrations (if applicable).
3.  **Start Backend:**
    ```bash
    cd server
    cargo run
    ```
4.  **Start Frontend:**
    ```bash
    cd react
    npm install
    npm run dev
    ```
