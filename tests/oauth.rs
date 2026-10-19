use std::sync::Arc;
use std::time::Duration;

use pgm_data::auth::TokenGate;
use pgm_data::error::Secret;
use pgm_data::secrets::{MemorySecrets, SecretStore, ACCOUNT_REFRESH, ACCOUNT_SECRET};
use pgm_data::sim::{Fixture, Plan};
use pgm_data::timeutil::ManualClock;

#[tokio::test]
async fn one_refresh_serves_concurrent_callers() {
    let fixture = Fixture::spawn().await;
    fixture
        .set_plan(Plan {
            token_delay: Duration::from_millis(200),
            ..Plan::default()
        })
        .await;
    let secrets = Arc::new(MemorySecrets::new());
    secrets
        .put(ACCOUNT_SECRET, &Secret::new("secret".into()).unwrap())
        .unwrap();
    let client =
        pgm_data::auth::build_client(Duration::from_secs(2), Duration::from_secs(2), 8).unwrap();
    let gate = Arc::new(TokenGate::new(
        client,
        fixture.token_url(),
        "client".into(),
        secrets,
        Arc::new(pgm_data::timeutil::SystemClock),
        60_000,
    ));
    let mut tasks = Vec::new();
    for _ in 0..12 {
        let gate = Arc::clone(&gate);
        tasks.push(tokio::spawn(async move {
            gate.access().await.unwrap().generation
        }));
    }
    for task in tasks {
        assert_eq!(task.await.unwrap(), 1);
    }
    assert_eq!(
        fixture.hits_token.load(std::sync::atomic::Ordering::SeqCst),
        1
    );
}

#[tokio::test]
async fn expiry_refreshes_once_and_rotates_the_refresh_token() {
    let fixture = Fixture::spawn().await;
    fixture
        .set_plan(Plan {
            expires_in: 3600,
            rotate_refresh: true,
            ..Plan::default()
        })
        .await;
    let secrets = Arc::new(MemorySecrets::new());
    secrets
        .put(ACCOUNT_SECRET, &Secret::new("secret".into()).unwrap())
        .unwrap();
    let clock = Arc::new(ManualClock::new(1_700_000_000_000));
    let client =
        pgm_data::auth::build_client(Duration::from_secs(2), Duration::from_secs(2), 4).unwrap();
    let gate = TokenGate::new(
        client,
        fixture.token_url(),
        "client".into(),
        Arc::clone(&secrets) as Arc<dyn pgm_data::secrets::SecretStore>,
        clock.clone(),
        60_000,
    );
    let first = gate.access().await.unwrap();
    assert_eq!(first.generation, 1);
    let stored = secrets.get(ACCOUNT_REFRESH).unwrap().unwrap();
    clock.advance(3_550_000);
    let second = gate.access().await.unwrap();
    assert_eq!(second.generation, 2);
    let rotated = secrets.get(ACCOUNT_REFRESH).unwrap().unwrap();
    assert_ne!(stored.expose(), rotated.expose());
    let hits = fixture.hits_token.load(std::sync::atomic::Ordering::SeqCst);
    let _ = gate.access().await.unwrap();
    assert_eq!(
        fixture.hits_token.load(std::sync::atomic::Ordering::SeqCst),
        hits
    );
}

#[tokio::test]
async fn stale_401_does_not_drop_a_newer_token() {
    let fixture = Fixture::spawn().await;
    let secrets = Arc::new(MemorySecrets::new());
    secrets
        .put(ACCOUNT_SECRET, &Secret::new("secret".into()).unwrap())
        .unwrap();
    let clock = Arc::new(ManualClock::new(1_700_000_000_000));
    let client =
        pgm_data::auth::build_client(Duration::from_secs(2), Duration::from_secs(2), 4).unwrap();
    let gate = TokenGate::new(
        client,
        fixture.token_url(),
        "client".into(),
        secrets,
        clock.clone(),
        1_000,
    );
    let first = gate.access().await.unwrap();
    clock.advance(3_700_000);
    let second = gate.access().await.unwrap();
    gate.invalidate(first.generation).await;
    let current = gate.access().await.unwrap();
    assert_eq!(current.generation, second.generation);
    assert_ne!(current.generation, first.generation);
}

#[tokio::test]
async fn repeated_401_does_not_loop() {
    let fixture = Fixture::spawn().await;
    fixture
        .set_plan(Plan {
            token_status: 401,
            ..Plan::default()
        })
        .await;
    let secrets = Arc::new(MemorySecrets::new());
    secrets
        .put(ACCOUNT_SECRET, &Secret::new("secret".into()).unwrap())
        .unwrap();
    let client =
        pgm_data::auth::build_client(Duration::from_secs(2), Duration::from_secs(2), 4).unwrap();
    let gate = Arc::new(TokenGate::new(
        client,
        fixture.token_url(),
        "client".into(),
        secrets,
        Arc::new(pgm_data::timeutil::SystemClock),
        1_000,
    ));
    assert!(gate.access().await.is_err());
    assert!(gate.access().await.is_err());
    let hits = fixture.hits_token.load(std::sync::atomic::Ordering::SeqCst);
    assert!(hits <= 2, "token endpoint hits {hits}");
}

#[tokio::test]
async fn invalid_grant_falls_back_to_client_credentials_once() {
    let fixture = Fixture::spawn().await;
    fixture
        .set_plan(Plan {
            reject_refresh: true,
            ..Plan::default()
        })
        .await;
    let secrets = Arc::new(MemorySecrets::new());
    secrets
        .put(ACCOUNT_SECRET, &Secret::new("secret".into()).unwrap())
        .unwrap();
    secrets
        .put(ACCOUNT_REFRESH, &Secret::new("refresh-old".into()).unwrap())
        .unwrap();
    let client =
        pgm_data::auth::build_client(Duration::from_secs(2), Duration::from_secs(2), 4).unwrap();
    let gate = TokenGate::new(
        client,
        fixture.token_url(),
        "client".into(),
        secrets,
        Arc::new(pgm_data::timeutil::SystemClock),
        1_000,
    );
    let issued = gate.access().await.unwrap();
    assert_eq!(issued.generation, 1);
    assert_eq!(
        fixture.hits_token.load(std::sync::atomic::Ordering::SeqCst),
        2
    );
}
