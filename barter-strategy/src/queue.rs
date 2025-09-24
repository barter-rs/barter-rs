use crate::{Result, StrategyError};
use async_trait::async_trait;
use fluvio::{
    Fluvio, FluvioConfig, Offset, PartitionConsumer, PartitionProducer, RecordKey, TopicProducer,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{error, info, warn};

#[derive(Debug, Clone)]
pub struct FluvioQueue {
    fluvio: Option<Fluvio>,
    test_mode: bool,
}

impl FluvioQueue {
    pub async fn new(test_mode: bool) -> Result<Self> {
        if test_mode {
            info!("Initializing Fluvio queue in test mode");
            return Ok(Self {
                fluvio: None,
                test_mode: true,
            });
        }

        // Try to connect to Fluvio cluster
        match Self::connect_to_fluvio().await {
            Ok(fluvio) => {
                info!("Successfully connected to Fluvio cluster");
                Ok(Self {
                    fluvio: Some(fluvio),
                    test_mode: false,
                })
            }
            Err(e) => {
                warn!("Failed to connect to Fluvio, running in test mode: {}", e);
                Ok(Self {
                    fluvio: None,
                    test_mode: true,
                })
            }
        }
    }

    async fn connect_to_fluvio() -> Result<Fluvio> {
        let config = FluvioConfig::load()
            .map_err(|e| StrategyError::Queue(format!("Failed to load Fluvio config: {}", e)))?;

        Fluvio::connect_with_config(&config)
            .await
            .map_err(|e| StrategyError::Queue(format!("Failed to connect to Fluvio: {}", e)))
    }

    pub async fn create_topics(&self) -> Result<()> {
        if self.test_mode {
            info!("Test mode: Skipping topic creation");
            return Ok(());
        }

        let topics = vec![
            "market-data",
            "processed-signals",
            "trading-decisions",
            "execution-results",
        ];

        if let Some(fluvio) = &self.fluvio {
            let admin = fluvio.admin().await;

            for topic_name in topics {
                match admin
                    .create(
                        topic_name.to_string(),
                        fluvio::metadata::topic::TopicSpec::new_computed(1, 1, None),
                    )
                    .await
                {
                    Ok(_) => info!("Created topic: {}", topic_name),
                    Err(e) => {
                        // Topic might already exist
                        warn!("Failed to create topic {}: {}", topic_name, e);
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn get_producer(&self, topic: &str) -> Result<Box<dyn MessageProducer>> {
        if self.test_mode {
            return Ok(Box::new(TestProducer::new(topic)));
        }

        if let Some(fluvio) = &self.fluvio {
            let producer = fluvio
                .topic_producer(topic)
                .await
                .map_err(|e| StrategyError::Queue(format!("Failed to create producer: {}", e)))?;

            Ok(Box::new(FluvioProducer { producer }))
        } else {
            Ok(Box::new(TestProducer::new(topic)))
        }
    }

    pub async fn get_consumer(&self, topic: &str) -> Result<Box<dyn MessageConsumer>> {
        if self.test_mode {
            return Ok(Box::new(TestConsumer::new(topic)));
        }

        if let Some(fluvio) = &self.fluvio {
            let consumer = fluvio
                .partition_consumer(topic, 0)
                .await
                .map_err(|e| StrategyError::Queue(format!("Failed to create consumer: {}", e)))?;

            Ok(Box::new(FluvioConsumer { consumer }))
        } else {
            Ok(Box::new(TestConsumer::new(topic)))
        }
    }
}

#[async_trait]
pub trait MessageProducer: Send + Sync {
    async fn send(&self, message: &[u8]) -> Result<()>;
    async fn send_with_key(&self, key: &str, message: &[u8]) -> Result<()>;
}

#[async_trait]
pub trait MessageConsumer: Send + Sync {
    async fn consume(&mut self) -> Result<Option<Vec<u8>>>;
    async fn consume_batch(&mut self, max_size: usize) -> Result<Vec<Vec<u8>>>;
}

struct FluvioProducer {
    producer: TopicProducer,
}

#[async_trait]
impl MessageProducer for FluvioProducer {
    async fn send(&self, message: &[u8]) -> Result<()> {
        self.producer
            .send(RecordKey::NULL, message)
            .await
            .map_err(|e| StrategyError::Queue(format!("Failed to send message: {}", e)))?;

        Ok(())
    }

    async fn send_with_key(&self, key: &str, message: &[u8]) -> Result<()> {
        self.producer
            .send(key, message)
            .await
            .map_err(|e| StrategyError::Queue(format!("Failed to send message: {}", e)))?;

        Ok(())
    }
}

struct FluvioConsumer {
    consumer: PartitionConsumer,
}

#[async_trait]
impl MessageConsumer for FluvioConsumer {
    async fn consume(&mut self) -> Result<Option<Vec<u8>>> {
        let mut stream = self.consumer.stream(Offset::end()).await
            .map_err(|e| StrategyError::Queue(format!("Failed to create stream: {}", e)))?;

        // Try to get one message with timeout
        match tokio::time::timeout(Duration::from_millis(100), stream.next()).await {
            Ok(Some(Ok(record))) => Ok(Some(record.value().to_vec())),
            Ok(Some(Err(e))) => Err(StrategyError::Queue(format!("Error consuming message: {}", e))),
            Ok(None) => Ok(None),
            Err(_) => Ok(None), // Timeout
        }
    }

    async fn consume_batch(&mut self, max_size: usize) -> Result<Vec<Vec<u8>>> {
        let mut messages = Vec::new();
        let mut stream = self.consumer.stream(Offset::end()).await
            .map_err(|e| StrategyError::Queue(format!("Failed to create stream: {}", e)))?;

        for _ in 0..max_size {
            match tokio::time::timeout(Duration::from_millis(10), stream.next()).await {
                Ok(Some(Ok(record))) => messages.push(record.value().to_vec()),
                Ok(Some(Err(e))) => {
                    error!("Error consuming message: {}", e);
                    break;
                }
                Ok(None) | Err(_) => break, // No more messages or timeout
            }
        }

        Ok(messages)
    }
}

// Test implementations for when Fluvio is not available
struct TestProducer {
    topic: String,
}

impl TestProducer {
    fn new(topic: &str) -> Self {
        Self {
            topic: topic.to_string(),
        }
    }
}

#[async_trait]
impl MessageProducer for TestProducer {
    async fn send(&self, message: &[u8]) -> Result<()> {
        info!(
            "Test mode: Would send message to topic '{}': {} bytes",
            self.topic,
            message.len()
        );
        Ok(())
    }

    async fn send_with_key(&self, key: &str, message: &[u8]) -> Result<()> {
        info!(
            "Test mode: Would send message to topic '{}' with key '{}': {} bytes",
            self.topic,
            key,
            message.len()
        );
        Ok(())
    }
}

struct TestConsumer {
    topic: String,
}

impl TestConsumer {
    fn new(topic: &str) -> Self {
        Self {
            topic: topic.to_string(),
        }
    }
}

#[async_trait]
impl MessageConsumer for TestConsumer {
    async fn consume(&mut self) -> Result<Option<Vec<u8>>> {
        // In test mode, return None (no messages)
        Ok(None)
    }

    async fn consume_batch(&mut self, _max_size: usize) -> Result<Vec<Vec<u8>>> {
        // In test mode, return empty batch
        Ok(Vec::new())
    }
}

// Message types for queue communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueMessage<T> {
    pub id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub data: T,
}

impl<T: Serialize> QueueMessage<T> {
    pub fn new(data: T) -> Self {
        Self {
            id: format!("msg_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap()),
            timestamp: chrono::Utc::now(),
            data,
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        serde_json::to_vec(self).map_err(|e| StrategyError::Serialization(e))
    }
}

impl<T: for<'de> Deserialize<'de>> QueueMessage<T> {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        serde_json::from_slice(bytes).map_err(|e| StrategyError::Serialization(e))
    }
}