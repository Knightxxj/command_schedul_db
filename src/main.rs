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
            Command::Add    => command_handle::command_add(args, db).await,
            Command::Done   => command_handle::command_done(db, args).await,
            Command::Remove => command_handle::command_remove(db, args).await,
            Command::Help   => command_handle::command_help(),
        },
        Err(e) => return Err(e),
    };

    Ok(())
}
