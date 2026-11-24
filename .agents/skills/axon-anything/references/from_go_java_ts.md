# Porting Go, Java, and TypeScript / Node.js to Pure Axon

## 1. Backend, Auth & Database Mapping

| Go / Java / TypeScript | Pure Axon (`v1.2.0`) |
|---|---|
| `if err != nil { return err }` / `try-catch` | `res := op() or return` (on `T ! i64` functions) |
| `&&`, `||`, `!` | `and`, `or`, `not` |
| Express / Gin / Spring Router | `use std::web` (`web_route_match`, `web_param`, `web_json`, `web_bearer_token`) |
| `jsonwebtoken` / `golang-jwt` | `use std::jwt` (`jwt_sign_hs256`, `jwt_verify_hs256`) |
| `argon2` / `bcrypt` / `otplib` / `casbin` | `use std::auth` (`argon2id_hash`, `argon2id_verify`, `totp_generate`, `totp_verify`, `RbacPolicy`) |
| OS Keychain / DPAPI / `crypto.randomBytes` | `use std::os_auth` (`os_csprng_bytes`, `os_dpapi_protect_hex`, `os_cred_write`) |
| `pg` / `pgx` (PostgreSQL 14–17 SCRAM) | `use std::postgres` (`pg_connect`, `pg_query`, `pg_encode_extended_query`) |
| `better-sqlite3` / `Knex` / `Flyway` | `use std::sqlite` + `use std::db_pool` (`SqlBuilder`, `SqlitePool`, `db_migrate_sqlite`) |
| `ioredis` / `go-redis` (RESP2/RESP3 + Pipeline) | `use std::redis` (`redis_connect`, `redis_get`/`set`/`hset`, `RedisPipeline`) |
| Embedded `BoltDB` / `LevelDB` | `use std::kv_acid` (`kv_open`, `kv_begin`, `kv_put`, `kv_commit`, `kv_rollback`, `kv_compact`) |
| `@grpc/grpc-js` / `protobufjs` | `use std::grpc` (`grpc_connect`, `grpc_unary_call`, `grpc_listen`, `pb_decode`) |

## 2. Eliminating Garbage-Collector Idioms
- Java/Go/TS code allocates short-lived closures and maps per request. In Axon:
  - Reuse `StrMap` / `DynMap` / `SqlBuilder` within handler scope and free deterministically (`smap_free`, `sqlb_free`).
  - Never concatenate strings in an `O(N^2)` loop (`s = s + chunk`); append to `b := buf_new(cap)` with `buf_push_str(b, chunk)` and finish with `buf_to_owned_str(move b)` in `O(N)`!
