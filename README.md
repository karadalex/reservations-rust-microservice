Reservation microservice in Rust
================================

## Instructions

```bash
cargo install sqlx-cli --no-default-features --features sqlite
sqlx migrate run --database-url sqlite://database.sqlite
cargo update
cargo run --bin init_db
cargo run
```

to add a new migration:
```bash
sqlx migrate add -r migration-name
```