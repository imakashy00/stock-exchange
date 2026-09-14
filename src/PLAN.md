# Plan: Rust Limit Order Book & Matching Engine
*(Rename to `plan.md` when you drop this in your project root — this is Project 1, build it first.)*

## Why this one, and why first
Real junior/entry postings in trading firms consistently ask for exactly this: "trading engines," "market making systems," order-book logic. It's also the *easier* of the two projects to start with — it's mostly data structures and algorithms in memory. No disk durability, no crash recovery, no binary file formats to get right. You can get a correct, testable core running in a few sessions before you ever touch networking.

## What it proves you can do
- Model financial state correctly (no float money bugs)
- Design and reason about a non-trivial concurrent data structure
- Write matching/allocation logic with edge cases (partial fills, self-trades, cancels)
- Build a service boundary around a core engine (gateway in/out)
- Understand how private trade confirmations (dropcopy) differ from public market data
- Think about risk checks before code executes financial side effects

## Architecture
```
Client/CLI ──> Gateway (validates order) ──> Matching Engine (core)
                                                  │
                    ┌────────────┬───────────────┼───────────────┬────────────┐
                    ▼            ▼                ▼               ▼
              Trade Log   Position/Ledger      Dropcopy       Market Data
                                            (private receipt   (public feed,
                                             back to trader)    broadcast)
```

## Build phases

**Phase 0 — Setup**
- `cargo new order-book-engine`
- Decide money representation up front: use `rust_decimal::Decimal`, never `f64`, for prices/quantities.

**Phase 1 — Core data model**
- `Order { id, side: Buy/Sell, kind: Limit/Market, price: Option<Decimal>, qty: Decimal, timestamp }`
- `Trade { buy_order_id, sell_order_id, price, qty, timestamp }`
- Unit test: struct construction, ordering by timestamp.

**Phase 2 — Order book structure**
- `BTreeMap<Decimal, VecDeque<Order>>` for bids (reverse-sorted) and asks — gives you price levels sorted automatically, FIFO within a level.
- Insert/remove/peek-best operations, each unit tested.

**Phase 3 — Matching algorithm**
- Price-time priority: a new order matches against the best-priced resting orders first, oldest first within a price level.
- Handle: full fill, partial fill, resting remainder, market orders (no price, sweep the book), cancels.
- This is the highest-value phase for interviews — write 10–15 unit tests covering each scenario explicitly (they double as your "here's how I know it's correct" talking points).

**Phase 4 — Trade execution & reporting**
- Every match emits a `Trade`, appended to an execution log (just a `Vec<Trade>` or a simple file append for now — no need for a real WAL yet, that's Project 2).

**Phase 5 — Dropcopy**
- For every trade, send a private confirmation back to each side's own account — separate from the public market data feed in Phase 8. Think of it as the "receipt" a trader gets for their own records, used to reconcile "what I think happened" against "what the exchange says happened."
- Implementation: a per-account queue (`tokio::sync::mpsc` works well, or a simple per-account log file) that receives a tagged copy of every `Trade` involving that account.
- The key distinction to internalize: dropcopy must never silently drop or duplicate a message for a given account (it's a private, reliable feed), while market data broadcast can be best-effort (it's public and high-volume). That distinction — reliable+private vs. best-effort+public — is exactly the kind of thing interviewers probe for.

**Phase 6 — Basic accounting**
- Per-account balances and positions. Reject/reduce orders that would overdraw a position — this is the "risk check" real desks care about.

**Phase 7 — Gateway**
- Wrap the engine behind a simple interface: a CLI REPL first, then a minimal TCP or `axum` HTTP endpoint that accepts order submissions and returns fills.
- This is where you introduce `tokio` and start thinking about concurrency (single writer to the book, multiple readers/submitters).

**Phase 8 — Market data feed**
- Broadcast top-of-book and trade prints over a `tokio::sync::broadcast` channel, or a tiny WebSocket if you want it demo-able.

**Phase 9 — Stretch goals (optional, do only if Phases 1–8 are solid)**
- Order types: IOC, FOK, stop-limit
- Cancel/replace
- Throughput/latency benchmark with `criterion`
- Property-based tests with `proptest` (e.g., "total quantity in the book never exceeds what was submitted")

## Testing strategy
- Unit test every matching scenario in isolation (Phase 3) before wiring in networking.
- Add one property test: conservation of quantity/value across any sequence of random order submissions.
- Keep a short `TESTING.md` note of what each test proves — this becomes your interview script.

## Resources

**Reference projects (real, found while researching this with you):**
- Orderbook-Based Mini-Exchange (gateway + matching engine + dropcopy + tickerplant): https://github.com/akulguptax/Orderbook-Based-Mini-Exchange
- Build a Fintech Platform in Rust — 3-part series (accounting → matching engine → API): https://github.com/ivanbgd/fintech_platform/blob/main/README.md
- `orderflow` — a small, readable matching engine experiment: https://github.com/dylanlott/orderflow
- "How I Built a High-Performance Copy Trading Bot in Rust" — good reference for the risk-check / circuit-breaker pattern in Phase 5: https://medium.com/@Teraus/how-i-built-a-high-performance-polymarket-copy-trading-bot-in-rust-and-how-you-can-use-it-to-e4df1acea791

**Book (concepts transfer even though it's C++):**
- *Building Low Latency Applications with C++* (O'Reilly) has a full chapter on building a matching engine — the data structures and matching logic map directly to Rust.

**Crates you'll want:**
- `rust_decimal` — fixed-point decimal math for money (never use `f64` for prices)
- `tokio` — async runtime for the gateway
- `axum` — minimal HTTP framework, if you go the REST route
- `serde` / `serde_json` — order/trade (de)serialization
- `criterion` — benchmarking, for the stretch goal
- `proptest` — property-based testing

## Interview talking points to prepare
- Why `Decimal` instead of `f64` for money
- Why `BTreeMap` + `VecDeque` gives you price-time priority "for free"
- Why dropcopy (private, reliable) and market data (public, best-effort) need different delivery guarantees even though both originate from the same trade event
- How you'd extend this to be multi-threaded safely (lock-free vs. single-writer-thread designs)
- What you'd add first for production readiness (persistence, replay, monitoring)