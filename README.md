# piazza

piazza turns Borsa Italiana's public share listings into structured data you can query locally.

The scraper discovers ISINs from the A-Z listings, fetches share pages concurrently with a configurable limit, handles temporary upstream failures with exponential backoff, and moves HTML parsing onto a Rayon pool so Tokio stays focused on I/O.
Parsed shares are stored in PostgreSQL, with a small Axum API for querying the result.

## What It Stores

For each share, the scraper stores the ISIN and name, plus the data found on the full detail page:

- basic instrument details
- market and segment information
- current price/trading data
- one-month, six-month, and one-year performance figures

The database schema lives in `migrations/`. The SQL used by the database crate is in `db/queries/`.

## Workspace Layout

```text
piazza/
├── app_config/      # config.toml, env vars, validation
├── db/              # PostgreSQL queries and inserts
├── scraper/         # HTTP fetching and HTML parsing
├── scraper_utils/   # scrape -> insert workflows
├── server/          # Axum HTTP API
└── src/main.rs      # scraper CLI entry point
```

## Technical Notes

Fetching pages is handled with `reqwest` on Tokio. Share pages are scraped concurrently, with a `Semaphore` limiting how many requests can be in flight at once. That limit is configured with `scraper.share_concurrency`, because the useful value depends a lot on the machine, network, and how much pressure the source site tolerates.

HTML parsing is pushed onto a Rayon thread pool. Parsing is CPU work, and keeping it off the async runtime avoids tying up Tokio worker threads while a page is being turned into the internal `Share` struct.

The HTTP client is reused and configured once through `ScraperRuntime`: connection pooling, request timeout, connect timeout, idle timeout, keepalive, and retry behavior all come from `config.toml`.

Requests go through an exponential backoff helper. The scraper retries responses that usually mean temporary pressure or upstream trouble, such as `429`, `502`, `503`, and `504`, but exits on statuses that look more permanent.

The ISIN crawler runs all A-Z letters concurrently. Each letter is paged until the scraper sees a repeated page signature, with `scraper.isin_max_pages_per_letter` as a safety cap so a markup change cannot make the crawl run forever.

Database inserts are run with `FuturesUnordered`. Share insertion uses a transaction per share so the details, market info, price data, and performance metrics are written as one unit.

## Setup

You need Rust and PostgreSQL.

Create a database, then point the app at it:

```sh
DATABASE_URL=postgres://user:password@localhost/piazza
```

The app reads `.env` from the project root, so for local use it is usually enough to put `DATABASE_URL` there.

Run the migrations:

```sh
sqlx migrate run
```

The rest of the runtime settings are in `config.toml`. The defaults are meant for local development: server on `127.0.0.1:3000`, scraper logs in `share_scraper.log`, server logs in `../server.log`, and a fairly high concurrent scrape limit.

## Scraping

The scraper CLI has three modes.

Fetch the A-Z list of shares and store their ISINs:

```sh
cargo run -- scrape-isins
```

Scrape all known shares and insert the detailed data:

```sh
cargo run -- scrape-shares
```

Refresh only shares older than the configured age:

```sh
cargo run -- refresh-shares
```

If no operation is passed, `scrape-shares` is used.

A normal first run is usually:

```sh
cargo run -- scrape-isins
cargo run -- scrape-shares
```

## API

Start the server with:

```sh
cargo run -p server
```

By default it listens on `http://127.0.0.1:3000`.

Available endpoints:

- `GET /all_isins` returns all known ISINs and names
- `GET /all_shares` returns all stored share records
- `GET /share` searches shares

`/share` accepts these optional query parameters:

- `isin`: exact ISIN match, for example `/share?isin=IT0003128367`
- `name`: case-insensitive name search, for example `/share?name=enel`
- `lang`: ISIN prefix filter, for example `/share?lang=IT`

Example:

```sh
curl 'http://127.0.0.1:3000/share?name=enel'
```

## Configuration

Configuration is loaded from `config.toml`, then environment variables can override it with the `PIAZZA__` prefix.

For example:

```sh
PIAZZA__SERVER__BIND_ADDRESS=127.0.0.1:4000 cargo run -p server
```

`DATABASE_URL` is also supported directly, because that is what most Rust/Postgres tooling expects.

Useful knobs in `config.toml`:

- `scraper.share_concurrency`: how many share pages can be scraped at once
- `scraper.share_refresh_age_minutes`: how old a record must be before `refresh-shares` picks it up
- `scraper.isin_max_pages_per_letter`: safety cap for crawling each A-Z listing page
- `logging.level`: default tracing level, overridden by `RUST_LOG`

## Notes

This scraper depends on the shape of Borsa Italiana pages. If the site changes its markup, parsing can break or start returning partial data.
