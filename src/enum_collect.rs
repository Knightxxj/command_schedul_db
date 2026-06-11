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
