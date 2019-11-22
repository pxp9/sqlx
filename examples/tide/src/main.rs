use futures::TryStreamExt;
use sqlx::{FromRow, Pool, Postgres};
use std::env;
use tide::error::ResultExt;
use tide::http::StatusCode;
use tide::response;
use tide::EndpointResult;
use tide::{App, Context};

// #[async_std::main]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv()?;

    let pool = Pool::<Postgres>::new(&env::var("DATABASE_URL")?).await?;

    run_migrations(&pool).await?;

    let mut app = App::with_state(pool);

    app.at("/v1/user").get(get_all_users).post(create_user);

    app.at("/v1/user/first").get(get_first_user);

    app.serve(("localhost", 8080))?;

    Ok(())
}

async fn run_migrations(mut pool: &Pool<Postgres>) -> anyhow::Result<()> {
    let _ = sqlx::query(
        r#"
CREATE TABLE IF NOT EXISTS users (
    id INT GENERATED ALWAYS AS IDENTITY,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    name TEXT NOT NULL
);
        "#,
    )
    .execute(&mut pool)
    .await?;

    Ok(())
}

async fn get_all_users(cx: Context<Pool<Postgres>>) -> EndpointResult {
    let mut pool = cx.state();

    let users: Vec<(i32, String)> = sqlx::query(r#"SELECT id, name FROM users"#)
        .fetch(&mut pool) // -> Stream<Row>
        .map_ok(FromRow::from_row)
        .try_collect()
        .await
        .server_err()?;

    Ok(response::json(users))
}

async fn get_first_user(cx: Context<Pool<Postgres>>) -> EndpointResult {
    let mut pool = cx.state();

    let (user_id,) /* : (i32,) */ = sqlx::query!(
        r#"
SELECT id
FROM users
ORDER BY created_at DESC
LIMIT 1
        "#,
    )
    .fetch_one(&mut pool)
    .await
    .server_err()?;

    Ok(response::json(vec![user_id]))
}

#[derive(serde::Deserialize)]
struct CreateUserRequest {
    name: String,
}

async fn create_user(mut cx: Context<Pool<Postgres>>) -> EndpointResult<StatusCode> {
    let req_body: CreateUserRequest = cx.body_json().await.client_err()?;

    let mut pool = cx.state();

    let _ = sqlx::query(r#"INSERT INTO users ( name ) VALUES ( $1 )"#)
        .bind(req_body.name)
        .execute(&mut pool)
        .await
        .server_err()?;

    Ok(StatusCode::CREATED)
}