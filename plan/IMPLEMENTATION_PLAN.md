# schedule_db 实施计划

> 从 JSONL 文件存储迁移到 PostgreSQL (Docker) + sqlx

---

## 一、架构对比

```
[旧] schedule (JSONL) /Rust/study/schedule/
  main.rs → command_handle.rs → file_handle.rs → schedule.jsonl
                                                    schedule_id.txt

[新] schedule_db (PostgreSQL) /Rust/study/schedule_db/
  main.rs → command_handle.rs → db.rs → PostgreSQL (Docker)
                                   ↑
                              config.rs (env vars)
```

---

## 二、文件变更清单

| 操作 | 文件 | 说明 |
|:--:|------|------|
| 新建 | `docker-compose.yml` | PostgreSQL 16 容器定义 |
| 新建 | `migrations/001_init.sql` | 建表 + 种子数据 |
| 新建 | `src/config.rs` | `SCHEDULE_DATABASE_URL` 环境变量 + 默认值 |
| 新建 | `src/db.rs` | 连接池初始化、migration 执行、全部 CRUD 查询 |
| 修改 | `Cargo.toml` | 替换依赖：移除 serde/serde_json，新增 sqlx/tokio/chrono |
| 修改 | `src/main.rs` | `#[tokio::main]` 异步入口 |
| 修改 | `src/command_handle.rs` | 全部 handler 改为 async，`file_handle::` → `db.` |
| 修改 | `src/struct_collect.rs` | 移除 `Serialize/Deserialize`，新增 `sqlx::FromRow` |
| 删除 | `src/file_handle.rs` | 由 `db.rs` 替代 |
| 不变 | `src/enum_collect.rs` | Command 枚举无需改动 |
| 可选 | `src/migrate.rs` | JSONL → PostgreSQL 数据迁移工具 |

---

## 三、Docker 环境

### docker-compose.yml

```yaml
services:
  db:
    image: postgres:16-alpine
    environment:
      POSTGRES_USER: schedule
      POSTGRES_PASSWORD: schedule
      POSTGRES_DB: schedule
    ports:
      - "5432:5432"
    volumes:
      - pgdata:/var/lib/postgresql/data

volumes:
  pgdata:
```

### 操作命令

```bash
docker compose up -d      # 启动（后台）
docker compose down       # 停止（保留数据卷）
docker compose down -v    # 停止并清除数据
docker compose logs db    # 查看日志
```

---

## 四、数据库 Schema

### migrations/001_init.sql

```sql
CREATE TABLE IF NOT EXISTS tasks (
    id         TEXT PRIMARY KEY,
    content    TEXT NOT NULL,
    tag        TEXT NOT NULL DEFAULT '',
    priority   TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS id_counter (
    id      INTEGER PRIMARY KEY DEFAULT 1 CHECK (id = 1),
    last_id TEXT NOT NULL DEFAULT 'task_00000'
);

INSERT INTO id_counter (id, last_id)
VALUES (1, 'task_00000')
ON CONFLICT (id) DO NOTHING;
```

关键设计决策：

| 决策 | 理由 |
|------|------|
| `id` 保持 `task_NNNNN` 格式 | 兼容已有数据模型，练手项目不引入新 ID 策略 |
| `id_counter` 只有一行，`CHECK (id = 1)` | 防止误插入第二行，保证计数器唯一 |
| `tag`/`priority` 默认空字符串 | 与现有 Record struct 一致 |
| `created_at`/`updated_at` 为 `TIMESTAMPTZ` | 带时区的时间戳，留待日后扩展 |
| 用 sqlx migrate 自动执行 | 无需手动跑 SQL，首次启动时自动建表 |

---

## 五、依赖变更 (Cargo.toml)

```toml
[package]
name = "schedule_db"
version = "0.1.0"
edition = "2021"          # 从 2024 降回 2021（sqlx 生态兼容性更好）

[dependencies]
sqlx = { version = "0.8", features = [
    "runtime-tokio",
    "postgres",
    "migrate",
    "chrono",
] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

| 移除 | 新增 | 原因 |
|------|------|------|
| `serde = "1.0"` | — | 不再需要序列化 |
| `serde_json = "1.0"` | — | 不再读写 JSONL |
| — | `sqlx = "0.8"` | PostgreSQL 驱动 + 编译期 SQL 校验 + migration |
| — | `tokio = "1"` | sqlx 所需的异步运行时 |
| `dirs = "5"` | — | 不再需要文件路径管理 |

**edition 从 `2024` 降回 `2021`**，因为 sqlx 的 `migrate!` 宏在 edition 2024 下可能有兼容性问题（截至 2026 年中），保守起见使用 2021。

---

## 六、逐文件实现细节

### 6.1 src/config.rs（新建）

```rust
use std::env;

pub struct AppConfig {
    pub database_url: String,
}

impl AppConfig {
    pub fn load() -> Self {
        let database_url = env::var("SCHEDULE_DATABASE_URL")
            .unwrap_or_else(|_| {
                "postgres://schedule:schedule@localhost:5432/schedule".to_string()
            });

        AppConfig { database_url }
    }
}
```

单职责：从环境变量或默认值提供数据库连接串。

### 6.2 src/db.rs（新建，核心）

```rust
use sqlx::postgres::PgPool;
use crate::struct_collect::Record;

pub struct Db {
    pool: PgPool,
}

impl Db {
    pub async fn connect(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPool::connect(database_url).await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(Db { pool })
    }

    // ── Counter ────────────────────────────────────

    pub async fn get_task_id(&self) -> Result<String, sqlx::Error> {
        let row: (String,) = sqlx::query_as(
            "SELECT last_id FROM id_counter WHERE id = 1"
        )
        .fetch_one(&self.pool).await?;
        Ok(row.0)
    }

    // ── Tasks ──────────────────────────────────────

    pub async fn list_tasks(
        &self,
        tag: Option<&str>,
        priority: Option<&str>,
    ) -> Result<Vec<Record>, sqlx::Error> {
        match (tag, priority) {
            (Some(t), Some(p)) => {
                sqlx::query_as::<_, Record>(
                    "SELECT id, content, tag, priority FROM tasks
                     WHERE tag = $1 AND priority = $2
                     ORDER BY id"
                )
                .bind(t).bind(p)
                .fetch_all(&self.pool).await
            }
            (Some(t), None) => {
                sqlx::query_as::<_, Record>(
                    "SELECT id, content, tag, priority FROM tasks
                     WHERE tag = $1 ORDER BY id"
                )
                .bind(t)
                .fetch_all(&self.pool).await
            }
            (None, Some(p)) => {
                sqlx::query_as::<_, Record>(
                    "SELECT id, content, tag, priority FROM tasks
                     WHERE priority = $1 ORDER BY id"
                )
                .bind(p)
                .fetch_all(&self.pool).await
            }
            (None, None) => {
                sqlx::query_as::<_, Record>(
                    "SELECT id, content, tag, priority FROM tasks ORDER BY id"
                )
                .fetch_all(&self.pool).await
            }
        }
    }

    pub async fn insert_task(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        record: &Record,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO tasks (id, content, tag, priority)
             VALUES ($1, $2, $3, $4)"
        )
        .bind(&record.id)
        .bind(&record.content)
        .bind(&record.tag)
        .bind(&record.priority)
        .execute(&mut **tx).await?;
        Ok(())
    }

    pub async fn mark_done(&self, id: &str) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "UPDATE tasks SET tag = 'done', updated_at = NOW() WHERE id = $1"
        )
        .bind(id)
        .execute(&self.pool).await?;
        Ok(result.rows_affected())
    }

    pub async fn delete_task(&self, id: &str) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM tasks WHERE id = $1"
        )
        .bind(id)
        .execute(&self.pool).await?;
        Ok(result.rows_affected())
    }

    // ── Transaction ────────────────────────────────

    pub async fn add_task_with_counter(
        &self,
        record: &Record,
        new_id: &str,
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("UPDATE id_counter SET last_id = $1 WHERE id = 1")
            .bind(new_id)
            .execute(&mut *tx).await?;

        Self::insert_task(&mut tx, record).await?;

        tx.commit().await?;
        Ok(())
    }
}
```

关键设计：

| 方法 | 对应旧逻辑 | 变化 |
|------|-----------|------|
| `get_task_id()` | 读 `schedule_id.txt` | — |
| `add_task_with_counter()` | 写 `schedule_id.txt` + 追加 JSONL | **合并为一个 DB 事务**，原子性从此根治 |
| `list_tasks()` | 读全 JSONL → 内存过滤 | 过滤下推到 SQL，只返回匹配行 |
| `mark_done()` | 读全 JSONL → 改一行 → 写全 JSONL | 单条 `UPDATE`，O(1) |
| `delete_task()` | 读全 JSONL → 过滤 → 写全 JSONL | 单条 `DELETE`，O(1) |

### 6.3 src/struct_collect.rs（修改）

```rust
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct Record {
    pub id: String,
    pub content: String,
    pub tag: String,
    pub priority: String,
}
```

变更：
- 移除 `serde::{Serialize, Deserialize}`
- 新增 `sqlx::FromRow`（自动将 SQL 行映射到 struct 字段）

### 6.4 src/main.rs（修改）

```rust
pub mod command_handle;
pub mod config;
pub mod db;
pub mod enum_collect;
pub mod struct_collect;

use config::AppConfig;
use db::Db;
use enum_collect::Command;
use std::env;
use std::str::FromStr;

#[tokio::main]
async fn main() {
    let config = AppConfig::load();

    let db = Db::connect(&config.database_url).await.unwrap_or_else(|e| {
        eprintln!("数据库连接失败: {}", e);
        eprintln!("请确认 Docker PostgreSQL 已启动: docker compose up -d");
        std::process::exit(1);
    });

    let args = env::args();
    if let Err(e) = run(&db, args).await {
        eprintln!("{}", e);
    }
}

async fn run(db: &Db, mut args: impl Iterator<Item = String>) -> Result<(), String> {
    args.next();

    let command_name = match args.next() {
        Some(arg) => arg,
        None => return Err("请输入有效命令！查看全部命令请输入[schedule help]".to_string()),
    };

    match Command::from_str(&command_name) {
        Ok(cmd) => match cmd {
            Command::List   => command_handle::command_list(db, args).await,
            Command::Add    => command_handle::command_add(db, args).await,
            Command::Done   => command_handle::command_done(db, args).await,
            Command::Remove => command_handle::command_remove(db, args).await,
            Command::Help   => command_handle::command_help(),
        },
        Err(e) => return Err(e),
    };

    Ok(())
}
```

变更：
- `fn main()` → `#[tokio::main] async fn main()`
- `fn run()` → `async fn run()`
- 新增 `config` / `db` 模块
- 移除 `file_handle` 模块
- 连接失败时给出明确的 Docker 提示

### 6.5 src/command_handle.rs（修改）

全部 handler 改为 `async fn`，参数从 `&FileStore` 改为 `&Db`。

#### command_list

```rust
pub async fn command_list(db: &Db, args: impl Iterator<Item = String>) {
    let filter = parse_filter_args(args);

    match db.list_tasks(
        filter.tag.as_deref(),
        filter.priority.as_deref(),
    ).await {
        Ok(tasks) => {
            for r in &tasks {
                println!(
                    "任务ID：{}, 任务内容：{}, 任务优先级：{}, 任务状态：{}",
                    r.id, r.content, r.priority, r.tag
                )
            }
        }
        Err(e) => eprintln!("查询失败: {}", e),
    }
}
```

#### command_add

```rust
pub async fn command_add(mut args: impl Iterator<Item = String>, db: &Db) {
    let current_id = match db.get_task_id().await {
        Ok(id) => id,
        Err(e) => {
            eprintln!("读取任务ID失败: {}", e);
            return;
        }
    };

    let new_id = match increment_task_id(&current_id) {
        Some(id) => id,
        None => {
            eprintln!("无法解析任务ID: {}", current_id);
            return;
        }
    };

    let content = match args.next() {
        Some(c) => c,
        None => {
            eprintln!("任务内容未提供！！");
            return;
        }
    };

    let filter = parse_filter_args(args);

    let record = Record {
        id: new_id.clone(),
        content,
        tag: filter.tag.unwrap_or_default(),
        priority: filter.priority.unwrap_or_default(),
    };

    if let Err(e) = db.add_task_with_counter(&record, &new_id).await {
        eprintln!("写入任务失败: {}", e);
        return;
    }

    println!("新增任务成功！");
    println!(
        "任务ID：{}, 任务内容：{}, 任务tag：{}, 任务优先级：{}",
        record.id, record.content, record.tag, record.priority
    );
}
```

#### command_done

```rust
pub async fn command_done(db: &Db, mut args: impl Iterator<Item = String>) {
    let task_id = match args.next() {
        Some(id) => id,
        None => {
            eprintln!("请提供要完成的任务ID");
            return;
        }
    };

    match db.mark_done(&task_id).await {
        Ok(0) => eprintln!("未找到ID为 {} 的任务", task_id),
        Err(e) => eprintln!("操作失败: {}", e),
        Ok(_) => println!("任务 {} 已标记为完成", task_id),
    }
}
```

#### command_remove

```rust
pub async fn command_remove(db: &Db, mut args: impl Iterator<Item = String>) {
    let task_id = match args.next() {
        Some(id) => id,
        None => {
            eprintln!("请提供要删除的任务ID");
            return;
        }
    };

    match db.delete_task(&task_id).await {
        Ok(0) => eprintln!("未找到ID为 {} 的任务", task_id),
        Err(e) => eprintln!("操作失败: {}", e),
        Ok(_) => println!("任务 {} 已删除", task_id),
    }
}
```

#### 保留不变的函数

- `command_help()` — 无变化
- `parse_filter_args()` — 无变化（纯参数解析，不涉及 I/O）
- `increment_task_id()` — 无变化（纯字符串操作）

### 6.6 src/enum_collect.rs（不变）

```rust
use std::str::FromStr;

#[derive(Debug)]
pub enum Command {
    List,
    Add,
    Done,
    Remove,
    Help,
}

impl FromStr for Command {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "list"   => Ok(Command::List),
            "add"    => Ok(Command::Add),
            "done"   => Ok(Command::Done),
            "remove" => Ok(Command::Remove),
            "help"   => Ok(Command::Help),
            _ => Err(format!("[schedule]不包含当前命令: {}", s)),
        }
    }
}
```

---

## 七、实施步骤（顺序严格）

### Step 1：创建项目骨架

```bash
mkdir -p schedule_db/migrations
mkdir -p schedule_db/src
```

### Step 2：写入 Docker Compose

创建 `docker-compose.yml` 如上。

### Step 3：写入 Migration

创建 `migrations/001_init.sql` 如上。

### Step 4：Cargo.toml

修改依赖清单。

### Step 5：新建文件

依次写入：
- `src/config.rs`
- `src/db.rs`

### Step 6：迁移现有文件

从原 `schedule/` 项目拷贝并修改：
- `src/main.rs` → 异步化 + 模块调整
- `src/command_handle.rs` → 异步化 + `file_handle::` → `db.`
- `src/struct_collect.rs` → FromRow 替换 Serialize/Deserialize
- `src/enum_collect.rs` → 直接拷贝

### Step 7：启动 Docker 并测试

```bash
cd schedule_db
docker compose up -d
cargo build
cargo run -- add "测试任务" --tag=start --priority=1
cargo run -- list
cargo run -- list --tag=start
cargo run -- done task_00001
cargo run -- remove task_00001
```

### Step 8：数据迁移（可选）

如果要从旧的 `schedule.jsonl` 迁移数据，写一个一次性脚本：

```rust
// src/bin/migrate.rs
// 读取 schedule.jsonl，逐行 INSERT 到 PostgreSQL
// 读取 schedule_id.txt，写入 id_counter 表
```

```bash
cargo run --bin migrate -- /path/to/old/schedule.jsonl /path/to/old/schedule_id.txt
```

---

## 八、关键收益

| 问题 | 旧方案 (JSONL) | 新方案 (PostgreSQL) |
|------|:--|:--|
| 原子性 | 两步写入无保护 | DB 事务保证 |
| 并发 | 无锁，多进程写坏数据 | 行锁 + 事务隔离 |
| done/remove 性能 | O(n) 全量读写 | O(1) 单条 UPDATE/DELETE |
| 首次运行 | 文件不存在 → 崩溃 | Migration 自动建表 |
| 查询性能 | 全量读到内存再过滤 | 过滤下推到 DB |
| 数据校验 | 无 | NOT NULL 约束 + 主键唯一 |
| 扩展性 | 新增字段需改所有行 | ALTER TABLE 即可 |

---

## 九、注意事项

1. **`cargo sqlx prepare`**：提交前运行此命令生成 `sqlx-data.json`，CI 环境无需连数据库即可编译。首次 `cargo build` 需要 PostgreSQL 在线（sqlx 编译时校验 SQL）。

2. **端口冲突**：确保本地 5432 端口未被占用，`docker compose.yml` 可改为其他端口。

3. **密码安全**：本方案是本地练手项目，密码硬编码在默认连接串里。生产环境务必使用环境变量或 secrets 管理。

4. **`migrate!` 宏路径**：sqlx 的 `migrate!("./migrations")` 宏在编译期相对 `CARGO_MANIFEST_DIR` 解析路径，确保 migrations 目录在项目根目录下。

5. **edition 2021 vs 2024**：使用 2021 以兼容 sqlx 宏。当 sqlx 完全支持 2024 后可升级。