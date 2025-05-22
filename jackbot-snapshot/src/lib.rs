use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::sync::Mutex;
use tokio::time;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RecordType {
    OrderBook,
    Trade,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DataRecord {
    pub exchange: String,
    pub market: String,
    pub record_type: RecordType,
    pub value: String,
}

#[derive(Debug, Default)]
pub struct FakeRedis {
    data: Mutex<Vec<DataRecord>>,
}

impl FakeRedis {
    pub async fn insert(&self, record: DataRecord) {
        self.data.lock().await.push(record);
    }

    pub async fn get_all(&self) -> Vec<DataRecord> {
        self.data.lock().await.clone()
    }
}

/// Serialize records to a pseudo-Parquet (newline-delimited JSON) file.
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
    fs::create_dir_all(s3_root)?;
    let dest = s3_root.join(file_name);
    fs::copy(local_path, &dest)?;
    Ok(dest)
}

fn cleanup_old_files(root: &Path, retention: Duration) -> io::Result<()> {
    if !root.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.is_file() {
            if let Ok(modified) = metadata.modified() {
                if SystemTime::now()
                    .duration_since(modified)
                    .unwrap_or_default()
                    > retention
                {
                    let _ = fs::remove_file(entry.path());
                }
            }
        } else if metadata.is_dir() {
            cleanup_old_files(&entry.path(), retention)?;
        }
    }
    Ok(())
}

#[derive(Serialize, Deserialize, Default)]
struct IcebergMeta {
    schema_version: u32,
    files: Vec<String>,
}

pub fn register_with_iceberg(metadata_path: &Path, file_path: &Path) -> io::Result<()> {
    let mut meta: IcebergMeta = if metadata_path.exists() {
        let data = fs::read_to_string(metadata_path)?;
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        IcebergMeta { schema_version: 1, files: Vec::new() }
    };
    meta.files.push(file_path.display().to_string());
    fs::write(metadata_path, serde_json::to_string(&meta)? )
}

#[derive(Clone)]
pub struct SnapshotConfig {
    pub interval: Duration,
    pub retention: Duration,
}

pub struct SnapshotScheduler {
    redis: Arc<FakeRedis>,
    s3_root: PathBuf,
    iceberg_metadata: PathBuf,
    config: SnapshotConfig,
}

impl SnapshotScheduler {
    pub fn new(redis: Arc<FakeRedis>, s3_root: PathBuf, iceberg_metadata: PathBuf, config: SnapshotConfig) -> Self {
        Self { redis, s3_root, iceberg_metadata, config }
    }

    pub async fn snapshot_once(&self) -> io::Result<()> {
        let records = self.redis.get_all().await;
        let file_name = format!("snapshot_{}.parquet", chrono::Utc::now().timestamp_millis());
        let local_path = std::env::temp_dir().join(&file_name);
        write_parquet(&records, &local_path)?;
        let (exchange, market) = records
            .get(0)
            .map(|r| (r.exchange.clone(), r.market.clone()))
            .unwrap_or_else(|| ("unknown".into(), "unknown".into()));
        let dest_dir = self.s3_root.join(&exchange).join(&market);
        let s3_path = upload_to_s3(&local_path, &dest_dir)?;
        register_with_iceberg(&self.iceberg_metadata, &s3_path)?;
        cleanup_old_files(&self.s3_root, self.config.retention)?;
        Ok(())
    }

    pub async fn start(&self) {
        let mut interval = time::interval(self.config.interval);
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
        redis
            .insert(DataRecord {
                exchange: "exch".into(),
                market: "btc-usd".into(),
                record_type: RecordType::OrderBook,
                value: "v".into(),
            })
            .await;
        let dir = std::env::temp_dir();
        let s3_root = dir.join("s3_test");
        let meta = dir.join("meta.json");
        let cfg = SnapshotConfig { interval: Duration::from_millis(1), retention: Duration::from_secs(0) };
        let scheduler = SnapshotScheduler::new(redis, s3_root.clone(), meta.clone(), cfg);
        scheduler.snapshot_once().await.unwrap();
        assert!(fs::read_dir(s3_root.join("exch/btc-usd")).unwrap().next().is_some());
        let meta_contents = fs::read_to_string(meta).unwrap();
        let meta: IcebergMeta = serde_json::from_str(&meta_contents).unwrap();
        assert_eq!(meta.files.len(), 1);
    }
}

