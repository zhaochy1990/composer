# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Composer is an AI agent orchestration platform that turns work items into parallelized, workflow-driven coding tasks. It pairs a Rust backend (Axum + SQLite) with a React frontend, built to ship as a single binary via rust-embed.

## Commands

### Development
```bash
pnpm install                # Install all dependencies
pnpm run dev                # Run both Vite dev server (port 5173) and Cargo watch concurrently
pnpm run dev:web            # Vite dev server only (port 5173, proxies /api to :3000)
pnpm run dev:server         # Cargo watch on Rust crates only
```

### Build
```bash
pnpm run build              # Build both web and server
pnpm run build:web          # Vite production build → packages/web/dist/
pnpm run build:server       # Cargo release build → target/release/composer-server
```

### Test

All tests live in the `tests/` directory at the project root:

```bash
cargo test --workspace      # All Rust tests (from tests/rust/)
cd packages/web && pnpm run test          # Frontend unit tests (Vitest, from tests/web/)
cd packages/web && pnpm run test:e2e      # Playwright E2E tests (from tests/e2e/)
cd packages/web && pnpm run test:e2e:ui   # Playwright with UI
cd packages/web && pnpm run test:e2e:headed # Playwright with visible browser
```

**Test directory structure:**
- `tests/rust/` — Cargo crate (`composer-tests`) with integration tests for all 6 Rust crates
- `tests/web/` — Vitest unit tests for React components, hooks, stores, and utilities
- `tests/e2e/` — Playwright E2E tests with fixtures and API helpers

### Lint & Format
```bash
pnpm run lint               # ESLint on web package
pnpm run format             # Prettier + cargo fmt
```

### Type Generation
```bash
pnpm run generate-types     # Export Rust types → packages/web/src/types/generated.ts
```

## Architecture

### Monorepo Structure
- **Cargo workspace** (`crates/`): 6 Rust crates + 1 test crate (`tests/rust/`)
- **pnpm workspace** (`packages/`): 1 React/TypeScript package
- **Tests** (`tests/`): All test code lives here, separated from production code

### Rust Crates (dependency order)
1. **api-types** — Shared structs/enums with `#[derive(TS)]` for TypeScript codegen via ts-rs
2. **db** — SQLx + SQLite layer (WAL mode). Migrations in `crates/db/migrations/`
3. **git** — Git worktree management for agent isolation (creates under `.composer/worktrees/`)
4. **executors** — Spawns Claude Code CLI processes, parses stream-JSON protocol, emits events
5. **services** — Business logic: TaskService, AgentService, SessionService, WorktreeService, EventBus
6. **server** — Axum HTTP server (port 3000), WebSocket hub, route handlers, embedded SPA serving

### Frontend (`packages/web/`)
- **React 18** with **TanStack Router** and **TanStack Query** for data fetching
- **Zustand** for client state, **shadcn/ui** (Radix + Tailwind) for components
- Path alias: `@/*` → `./src/*`
- Two main views: TaskBoard (kanban) and AgentPool (agent management)

### Data Flow
Frontend (React) → REST API (TanStack Query) → Axum routes → Services → DB + Executors + EventBus → WebSocket → Frontend

### Key Patterns
- **Rust→TS types**: Rust structs with `#[derive(TS)]` auto-generate `packages/web/src/types/generated.ts`
- **Event broadcasting**: tokio broadcast channel (EventBus) pushes `WsEvent` over WebSocket to frontend
- **Agent isolation**: Each agent session gets its own git worktree under `.composer/worktrees/`
- **Process management**: `AgentProcessManager` uses DashMap for lock-free tracking of spawned Claude Code processes
- **Single binary**: Production build embeds the React SPA into the Rust binary via rust-embed

### API Routes
All routes are under the Axum server at `:3000`:
- `/health` — Health check
- `/tasks` — CRUD + `/tasks/{id}/assign`, `/tasks/{id}/move`, `/tasks/{id}/sessions`
- `/agents` — CRUD + `/agents/{id}/health`, `/agents/discover`
- `/sessions` — Create, get, resume, logs
- `/worktrees` — List, delete
- `/ws` — WebSocket (bidirectional: WsCommand/WsEvent)

### Environment Variables
- `DATABASE_URL` — default: `sqlite:composer.db?mode=rwc`
- `CORS_ORIGINS` — default: localhost origins for ports 5173 and 3000
- `RUST_LOG` — default: `composer=debug,tower_http=debug`

## Development Workflow

When adding a new feature, the typical flow is:
1. Define types in `crates/api-types/` (with `#[derive(TS)]`)
2. Add migration in `crates/db/migrations/`
3. Add DB model methods in `crates/db/src/models/`
4. Implement service logic in `crates/services/src/`
5. Add route handlers in `crates/server/src/routes/`
6. Run `pnpm run generate-types` to sync types to frontend
7. Build React components in `packages/web/src/components/`
8. Add TanStack Query hooks in `packages/web/src/hooks/`
9. Add tests in `tests/` (Rust tests in `tests/rust/tests/`, frontend tests in `tests/web/`)

**Important:** Do not add `#[cfg(test)]` blocks in `crates/` source files or `__tests__/` directories in `packages/web/src/`. All tests belong in the `tests/` directory.

## E2E Tests

Located in `tests/e2e/tests/`. Uses Playwright with a test-specific DB (`sqlite:composer_test.db`). Tests run sequentially (1 worker). The Playwright config (`packages/web/playwright.config.ts`) auto-starts both the Cargo server and Vite dev server.
