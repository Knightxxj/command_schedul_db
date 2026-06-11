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
