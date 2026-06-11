use crate::db::Db;
use crate::struct_collect::Record;

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

pub fn command_help() {
    println!("schedule -- 终端任务管理工具");
    println!();
    println!("用法：schedule <命令> [参数]");
    println!();
    println!("命令：");
    println!("  list   [--tag=<标签>] [--priority=<优先级>]   列出任务，可选按标签/优先级过滤");
    println!("  add    <内容> [--tag=<标签>] [--priority=<优先级>]");
    println!("                             添加新任务");
    println!("  done   <任务ID>            标记任务为完成");
    println!("  remove <任务ID>            删除指定任务");
    println!("  help                       显示帮助信息");
}

struct FilterParams {
    tag: Option<String>,
    priority: Option<String>,
}

fn parse_filter_args(args: impl Iterator<Item = String>) -> FilterParams {
    let mut tag: Option<String> = None;
    let mut priority: Option<String> = None;
    let mut positional = 0;

    for arg in args {
        if let Some(stripped) = arg.strip_prefix("--") {
            if let Some((key, val)) = stripped.split_once('=') {
                match key {
                    "tag" => tag = Some(val.to_string()),
                    "priority" => priority = Some(val.to_string()),
                    _ => eprintln!("忽略未知参数: --{}", key),
                }
            }
        } else {
            match positional {
                0 => tag = Some(arg),
                1 => priority = Some(arg),
                _ => {}
            }
            positional += 1;
        }
    }

    FilterParams { tag, priority }
}

fn increment_task_id(s: &str) -> Option<String> {
    let underscore_pos = s.rfind('_')?;
    let (prefix, num_str) = s.split_at(underscore_pos + 1);
    let num: u32 = num_str.parse().ok()?;
    let new_num = num + 1;
    let width = num_str.len();
    let new_num_str = format!("{:0width$}", new_num);
    Some(format!("{}{}", prefix, new_num_str))
}
