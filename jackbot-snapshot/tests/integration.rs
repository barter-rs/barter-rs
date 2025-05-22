use jackbot_snapshot::{FakeRedis, SnapshotScheduler};
use std::{path::PathBuf, sync::Arc, time::Duration};

#[tokio::test]
async fn test_scheduler_multiple_snapshots() {
    let redis = Arc::new(FakeRedis::default());
    redis.insert("k1", "v1").await;
    let dir = std::env::temp_dir();
    let s3_root = dir.join("s3_integration");
    let meta = dir.join("meta_integration.json");
    let scheduler = SnapshotScheduler::new(redis, s3_root.clone(), meta.clone(), Duration::from_millis(1));

    // Take two snapshots manually
    scheduler.snapshot_once().await.unwrap();
    scheduler.snapshot_once().await.unwrap();

    let files: Vec<_> = std::fs::read_dir(&s3_root).unwrap().collect();
    assert_eq!(files.len(), 2);
    let data: Vec<String> = serde_json::from_str(&std::fs::read_to_string(meta).unwrap()).unwrap();
    assert_eq!(data.len(), 2);
}
