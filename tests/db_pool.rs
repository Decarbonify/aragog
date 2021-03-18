extern crate aragog;

use serde::{Deserialize, Serialize};

use aragog::{
    AuthMode, DatabaseAccess, DatabaseConnectionPool, DatabaseRecord, OperationOptions, Record,
    ServiceError,
};
use common::*;

pub mod common;

#[derive(Serialize, Deserialize, Record, Clone)]
#[hook(before_create(func = "before_create"))]
struct Dish {
    pub name: String,
    pub price: u16,
}

impl Dish {
    fn before_create(&self) -> Result<(), ServiceError> {
        Err(ServiceError::InternalError {
            message: String::from("Hook forbids creation").into(),
        })
    }
}

#[maybe_async::test(
    feature = "blocking",
    async(all(not(feature = "blocking")), tokio::test)
)]
async fn works_with_correct_parameters() {
    DatabaseConnectionPool::builder()
        .with_credentials(
            &std::env::var("DB_HOST").unwrap_or(DEFAULT_DB_HOST.to_string()),
            &std::env::var("DB_NAME").unwrap_or(DEFAULT_DB_NAME.to_string()),
            &std::env::var("DB_USER").unwrap_or(DEFAULT_DB_USER.to_string()),
            &std::env::var("DB_PWD").unwrap_or(DEFAULT_DB_PWD.to_string()),
        )
        .with_schema_path("./tests/schema.yaml")
        .apply_schema()
        .build()
        .await
        .unwrap();
}

#[maybe_async::test(
    feature = "blocking",
    async(all(not(feature = "blocking")), tokio::test)
)]
async fn fails_with_wrong_parameters() {
    match DatabaseConnectionPool::builder()
        .with_credentials(
            &std::env::var("DB_HOST").unwrap_or(DEFAULT_DB_HOST.to_string()),
            &std::env::var("DB_NAME").unwrap_or(DEFAULT_DB_NAME.to_string()),
            "fake_user",
            "fake_password",
        )
        .with_schema_path("./tests/schema.yaml")
        .with_auth_mode(AuthMode::Basic)
        .apply_schema()
        .build()
        .await
    {
        Ok(_) => panic!("should have failed"),
        Err(e) => match e {
            ServiceError::ArangoError(db_error) => assert_eq!(db_error.http_error.http_code(), 401),
            _ => panic!("wrong error"),
        },
    }
    // TODO: Remove comments when https://github.com/fMeow/arangors/issues/69 is closed
    // match DatabaseConnectionPool::builder()
    //     .with_credentials(
    //         &std::env::var("DB_HOST").unwrap_or(DEFAULT_DB_HOST.to_string()),
    //         &std::env::var("DB_NAME").unwrap_or(DEFAULT_DB_NAME.to_string()),
    //         "fake_user",
    //         "fake_password",
    //     )
    //     .with_schema_path("./tests/schema.yaml")
    //     .with_auth_mode(AuthMode::Jwt)
    //     .apply_schema()
    //     .build()
    //     .await {
    //     Ok(_) => panic!("should have failed"),
    //     Err(e) => match e {
    //         ServiceError::ArangoError(db_error) => assert_eq!(db_error.http_error.http_code(), 401),
    //         _ => panic!("wrong error")
    //     }
    // }
}

#[maybe_async::test(
    feature = "blocking",
    async(all(not(feature = "blocking")), tokio::test)
)]
async fn operation_options() {
    let pool = DatabaseConnectionPool::builder()
        .with_credentials(
            &std::env::var("DB_HOST").unwrap_or(DEFAULT_DB_HOST.to_string()),
            &std::env::var("DB_NAME").unwrap_or(DEFAULT_DB_NAME.to_string()),
            &std::env::var("DB_USER").unwrap_or(DEFAULT_DB_USER.to_string()),
            &std::env::var("DB_PWD").unwrap_or(DEFAULT_DB_PWD.to_string()),
        )
        .with_schema_path("./tests/schema.yaml")
        .apply_schema()
        .with_operation_options(
            OperationOptions::default()
                .wait_for_sync(true)
                .ignore_hooks(true),
        )
        .build()
        .await
        .unwrap();
    let options = pool.operation_options();
    assert_eq!(options.wait_for_sync, Some(true));
    assert_eq!(options.ignore_revs, true);
    assert_eq!(options.ignore_hooks, true);
    // the hook is not launched
    DatabaseRecord::create(
        Dish {
            name: "Cordon Bleu".to_string(),
            price: 7,
        },
        &pool,
    )
    .await
    .unwrap();
    // The hook is launched manually
    match DatabaseRecord::create_with_options(
        Dish {
            name: "Cordon Bleu".to_string(),
            price: 7,
        },
        &pool,
        OperationOptions::default(),
    )
    .await
    {
        Ok(_) => panic!("Hook should have launched failure"),
        Err(e) => match e {
            ServiceError::InternalError { message } => {
                assert_eq!(message.unwrap(), "Hook forbids creation".to_string())
            }
            _ => panic!("Wrong error"),
        },
    }
}