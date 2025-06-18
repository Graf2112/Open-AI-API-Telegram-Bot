use sqlx::{migrate::MigrateDatabase, Error, Pool, Sqlite, SqlitePool};

pub async fn init_db() -> Result<Pool<Sqlite>, Error> {
    if !Sqlite::database_exists("db.sqlite").await.unwrap_or(false) {
        Sqlite::create_database("db.sqlite").await?;
    }

    let db = SqlitePool::connect("db.sqlite").await;
    if let Ok(db) = db {
        let query_res = sqlx::query(
            "CREATE TABLE IF NOT EXISTS context (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                message TEXT NOT NULL,
                responder TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&db)
        .await;

        if let Err(err) = query_res {
            println!("Failed to create table 1: {:?}", err);
            return Err(err);
        }
        let query_res = sqlx::query(
            "CREATE TABLE IF NOT EXISTS users (
                user_id INTEGER PRIMARY KEY NOT NULL,
                system TEXT,
                temperature FLOAT,
                context_len INTEGER NOT NULL
            )",
        )
        .execute(&db)
        .await;

        if let Err(err) = query_res {
            println!("Failed to create table 2: {:?}", err);
            return Err(err);
        }
        return Ok(db);
    } else {
        let err = db.err().unwrap();
        println!("Failed to connect to database: {:?}", err);
        return Err(err);
    }
}
