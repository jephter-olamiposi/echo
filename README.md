# 📋 Echo

**Universal Clipboard Sync** — Copy on one device, paste on any other. Instantly.

![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)
![TypeScript](https://img.shields.io/badge/TypeScript-007ACC?style=flat&logo=typescript&logoColor=white)
![Tauri](https://img.shields.io/badge/Tauri-FFC131?style=flat&logo=tauri&logoColor=black)
![React](https://img.shields.io/badge/React-20232A?style=flat&logo=react&logoColor=61DAFB)

## ✨ Features

- 🔄 **Real-time Sync** — Clipboard changes sync instantly across all devices
- 🔐 **End-to-End Encryption** — Optional AES-256-GCM encryption (your passphrase never leaves your device)
- 📱 **QR Code Device Linking** — Scan to connect new devices in seconds
- 🖥️ **Cross-Platform** — macOS, Windows, Linux (mobile coming soon)
- 📜 **Clipboard History** — Access your last 50 clipboard items
- ⚡ **Low Latency** — WebSocket-based for sub-second sync
- 🛡️ **Rate Limiting** — Built-in protection against abuse

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Echo Architecture                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│   ┌──────────┐     ┌──────────┐     ┌──────────┐                │
│   │ Device A │     │ Device B │     │ Device C │                │
│   │ (macOS)  │     │ (Windows)│     │ (Mobile) │                │
│   └────┬─────┘     └────┬─────┘     └────┬─────┘                │
│        │                │                │                       │
│        │    WebSocket   │    WebSocket   │                       │
│        └───────────┬────┴────────────────┘                       │
│                    │                                              │
│              ┌─────▼─────┐                                       │
│              │   Echo    │  Hub-and-Spoke Model                  │
│              │  Backend  │  (Rust + Axum)                        │
│              └─────┬─────┘                                       │
│                    │                                              │
│              ┌─────▼─────┐                                       │
│              │ PostgreSQL│                                       │
│              │    DB     │                                       │
│              └───────────┘                                       │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

## 🚀 Quick Start

### Prerequisites

- [Rust](https://rustup.rs/) (1.70+)
- [Node.js](https://nodejs.org/) (18+)
- [PostgreSQL](https://www.postgresql.org/) (14+)

### 1. Clone & Setup Database

```bash
git clone https://github.com/jephter-olamiposi/echo.git
cd echo

# Create database
createdb echo_db
```

### 2. Configure Environment

```bash
# Backend
cp backend/.env.example backend/.env
# Edit backend/.env with your database credentials

# Frontend (optional, for production URLs)
cp desktop/.env.example desktop/.env
```

### 3. Run Migrations

```bash
cd backend
cargo install sqlx-cli
sqlx migrate run
```

### 4. Start Backend

```bash
cd backend
cargo run
# Server starts at http://localhost:3000
```

### 5. Start Desktop App

```bash
cd desktop
npm install
npm run tauri dev
```

## 📱 Device Linking

Echo uses QR codes for easy device pairing:

1. **Open Echo** on your primary device
2. Click **"📱 Link Device"** to show QR code
3. **Scan the QR code** with your mobile/secondary device
4. Devices are now synced!

The QR code contains your session token and server URLs, allowing instant connection without manual login on new devices.

## 🔐 End-to-End Encryption

Echo supports optional E2EE using AES-256-GCM:

1. Click **"🔐 Enable E2EE"** in the dashboard
2. Enter a **passphrase** (use the same passphrase on all devices)
3. All clipboard data is now encrypted client-side

**Security Details:**
- Key derivation: PBKDF2 with 100,000 iterations
- Encryption: AES-256-GCM with random nonces
- The server only sees encrypted ciphertext
- Passphrase never leaves your device

## 📁 Project Structure

```
echo/
├── backend/              # Rust API server
│   ├── src/
│   │   ├── main.rs       # Entry point, router setup
│   │   ├── handler.rs    # HTTP & WebSocket handlers
│   │   ├── state.rs      # AppState, SyncEngine
│   │   ├── models.rs     # Request/response types
│   │   ├── middleware.rs # Auth middleware
│   │   ├── error.rs      # Error handling
│   │   └── tests.rs      # Unit tests
│   └── migrations/       # SQL migrations
├── desktop/              # Tauri desktop app
│   ├── src/              # React frontend
│   │   ├── App.tsx       # Main UI
│   │   ├── crypto.ts     # E2EE utilities
│   │   ├── auth.ts       # Token storage
│   │   └── config.ts     # Environment config
│   └── src-tauri/        # Rust backend
│       └── src/
│           ├── lib.rs    # Tauri setup
│           └── clipboard.rs  # Clipboard monitoring
└── README.md
```

## 🔧 Configuration

### Backend (`backend/.env`)

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection string | Required |
| `JWT_SECRET` | Secret for JWT signing | Required |
| `RUST_LOG` | Log level (debug, info, warn, error) | `debug` |

### Frontend (`desktop/.env`)

| Variable | Description | Default |
|----------|-------------|---------|
| `VITE_API_URL` | Backend API URL | `http://localhost:3000` |
| `VITE_WS_URL` | WebSocket URL | `ws://localhost:3000` |

## 🧪 Running Tests

```bash
cd backend
cargo test
```

## 🚢 Deployment

### Backend (Railway/Fly.io)

1. Set environment variables in your hosting platform
2. Deploy the `backend/` directory
3. Run migrations: `sqlx migrate run`

### Desktop App

```bash
cd desktop
npm run tauri build
# Outputs to desktop/src-tauri/target/release/bundle/
```

## 🛣️ Roadmap

- [ ] Mobile apps (iOS/Android via Tauri)
- [ ] Image/file clipboard sync
- [ ] Clipboard sharing between users
- [ ] Browser extension
- [ ] Self-hosted Docker deployment

## 📄 License

MIT © [Jephter Olamiposi](https://github.com/jephter-olamiposi)

---

**Built with ❤️ using Rust, TypeScript, and Tauri**
