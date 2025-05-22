use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::sync::Mutex;
use tokio::time;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DataRecord {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Default)]
pub struct FakeRedis {
    data: Mutex<HashMap<String, String>>, 
}

impl FakeRedis {
    pub async fn insert(&self, key: impl Into<String>, value: impl Into<String>) {
        self.data.lock().await.insert(key.into(), value.into());
    }

    pub async fn get_all(&self) -> Vec<DataRecord> {
        self.data
            .lock()
            .await
            .iter()
            .map(|(k, v)| DataRecord {
                key: k.clone(),
                value: v.clone(),
            })
            .collect()
    }
}

pub fn write_parquet(records: &[DataRecord], path: &Path) -> io::Result<()> {
    let mut file = File::create(path)?;
    for record in records {
        serde_json::to_writer(&mut file, record)?;
        file.write_all(b"\n")?;
    }
    Ok(())
}

pub fn upload_to_s3(local_path: &Path, s3_root: &Path) -> io::Result<PathBuf> {
    let file_name = local_path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::Other, "missing file name"))?;
    let dest = s3_root.join(file_name);
    fs::create_dir_all(s3_root)?;
    fs::copy(local_path, &dest)?;
    Ok(dest)
}

pub fn register_with_iceberg(metadata_path: &Path, file_path: &Path) -> io::Result<()> {
    let mut entries: Vec<String> = if metadata_path.exists() {
        let data = fs::read_to_string(metadata_path)?;
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        Vec::new()
    };
    entries.push(file_path.display().to_string());
    fs::write(metadata_path, serde_json::to_string(&entries)? )
}

pub struct SnapshotScheduler {
    redis: Arc<FakeRedis>,
    s3_root: PathBuf,
    iceberg_metadata: PathBuf,
    interval: Duration,
}

impl SnapshotScheduler {
    pub fn new(redis: Arc<FakeRedis>, s3_root: PathBuf, iceberg_metadata: PathBuf, interval: Duration) -> Self {
        Self { redis, s3_root, iceberg_metadata, interval }
    }

    pub async fn snapshot_once(&self) -> io::Result<()> {
        let records = self.redis.get_all().await;
        let file_name = format!("snapshot_{}.parquet", chrono::Utc::now().timestamp_millis());
        let local_path = std::env::temp_dir().join(&file_name);
        write_parquet(&records, &local_path)?;
        let s3_path = upload_to_s3(&local_path, &self.s3_root)?;
        register_with_iceberg(&self.iceberg_metadata, &s3_path)?;
        Ok(())
    }

    pub async fn start(&self) {
        let mut interval = time::interval(self.interval);
        loop {
            interval.tick().await;
            if let Err(err) = self.snapshot_once().await {
                eprintln!("snapshot failed: {err}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_snapshot_once() {
        let redis = Arc::new(FakeRedis::default());
        redis.insert("k", "v").await;
        let dir = std::env::temp_dir();
        let s3_root = dir.join("s3_test");
        let meta = dir.join("meta.json");
        let scheduler = SnapshotScheduler::new(redis, s3_root.clone(), meta.clone(), Duration::from_millis(1));
        scheduler.snapshot_once().await.unwrap();
        assert!(fs::read_dir(&s3_root).unwrap().next().is_some());
        let data: Vec<String> = serde_json::from_str(&fs::read_to_string(meta).unwrap()).unwrap();
        assert_eq!(data.len(), 1);
    }
}

