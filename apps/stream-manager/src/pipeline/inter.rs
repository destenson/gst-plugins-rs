use gst::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use thiserror::Error;
use tracing::{debug, error, info};

#[derive(Debug, Error)]
pub enum InterPipelineError {
    #[error("Failed to create element: {0}")]
    ElementCreation(String),
    #[error("Failed to add element to pipeline")]
    PipelineAddError,
    #[error("Failed to link elements")]
    LinkError,
    #[error("Producer not found: {0}")]
    ProducerNotFound(String),
    #[error("Consumer already registered: {0}")]
    ConsumerAlreadyExists(String),
    #[error("Connection not found: {0}")]
    ConnectionNotFound(String),
    #[error("Pipeline error")]
    PipelineError(#[from] gst::StateChangeError),
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

#[derive(Debug)]
pub struct InterConnection {
    pub producer_id: String,
    pub consumer_pipelines: Vec<gst::Pipeline>,
    pub state: ConnectionState,
    pub intersink: Option<gst::Element>,
    pub intersrcs: Vec<gst::Element>,
}

impl InterConnection {
    pub fn new(producer_id: String) -> Self {
        Self {
            producer_id,
            consumer_pipelines: Vec::new(),
            state: ConnectionState::Disconnected,
            intersink: None,
            intersrcs: Vec::new(),
        }
    }
}

pub struct InterPipelineManager {
    connections: Arc<RwLock<HashMap<String, InterConnection>>>,
}

impl InterPipelineManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a producer with an intersink element
    pub fn register_producer(
        &self,
        producer_id: &str,
        pipeline: &gst::Pipeline,
    ) -> Result<gst::Element, InterPipelineError> {
        info!("Registering inter-pipeline producer: {}", producer_id);

        // Create intersink element
        let intersink = gst::ElementFactory::make("intersink")
            .name(&format!("intersink-{}", producer_id))
            .property("producer-name", producer_id)
            .build()
            .map_err(|_| InterPipelineError::ElementCreation("intersink".to_string()))?;

        // Add to pipeline
        pipeline
            .add(&intersink)
            .map_err(|_| InterPipelineError::PipelineAddError)?;

        // Store connection
        let mut connections = self.connections.write().unwrap();
        let mut connection = InterConnection::new(producer_id.to_string());
        connection.intersink = Some(intersink.clone());
        connection.state = ConnectionState::Connecting;
        connections.insert(producer_id.to_string(), connection);

        debug!("Producer {} registered successfully", producer_id);
        Ok(intersink)
    }

    /// Create a consumer pipeline with an intersrc element
    pub fn create_consumer_pipeline(
        &self,
        producer_id: &str,
        consumer_name: &str,
    ) -> Result<(gst::Pipeline, gst::Element), InterPipelineError> {
        info!(
            "Creating consumer pipeline {} for producer {}",
            consumer_name, producer_id
        );

        // Check if producer exists
        {
            let connections = self.connections.read().unwrap();
            if !connections.contains_key(producer_id) {
                return Err(InterPipelineError::ProducerNotFound(
                    producer_id.to_string(),
                ));
            }
        }

        // Create new pipeline for consumer
        let pipeline = gst::Pipeline::builder()
            .name(&format!("consumer-{}-{}", producer_id, consumer_name))
            .build();

        // Create intersrc element
        let intersrc = gst::ElementFactory::make("intersrc")
            .name(&format!("intersrc-{}-{}", producer_id, consumer_name))
            .property("producer-name", producer_id)
            .build()
            .map_err(|_| InterPipelineError::ElementCreation("intersrc".to_string()))?;

        // Add to consumer pipeline
        pipeline
            .add(&intersrc)
            .map_err(|_| InterPipelineError::PipelineAddError)?;

        // Update connection with consumer info
        let mut connections = self.connections.write().unwrap();
        if let Some(connection) = connections.get_mut(producer_id) {
            connection.consumer_pipelines.push(pipeline.clone());
            connection.intersrcs.push(intersrc.clone());
            connection.state = ConnectionState::Connected;
        }

        debug!(
            "Consumer pipeline {} created for producer {}",
            consumer_name, producer_id
        );

        Ok((pipeline, intersrc))
    }

    /// Connect a producer element to an intersink
    pub fn connect_producer_element(
        &self,
        producer_id: &str,
        source_element: &gst::Element,
    ) -> Result<(), InterPipelineError> {
        let connections = self.connections.read().unwrap();
        let connection = connections
            .get(producer_id)
            .ok_or_else(|| InterPipelineError::ProducerNotFound(producer_id.to_string()))?;

        let intersink = connection
            .intersink
            .as_ref()
            .ok_or_else(|| InterPipelineError::InvalidConfig("No intersink found".to_string()))?;

        source_element
            .link(intersink)
            .map_err(|_| InterPipelineError::LinkError)?;

        info!("Connected source element to producer {}", producer_id);
        Ok(())
    }

    /// Disconnect and cleanup a consumer
    pub fn disconnect_consumer(
        &self,
        producer_id: &str,
        consumer_pipeline: &gst::Pipeline,
    ) -> Result<(), InterPipelineError> {
        info!(
            "Disconnecting consumer from producer {}",
            producer_id
        );

        // Set consumer pipeline to NULL state
        consumer_pipeline
            .set_state(gst::State::Null)
            .map_err(|_| InterPipelineError::PipelineError(gst::StateChangeError))?;

        // Remove from tracking
        let mut connections = self.connections.write().unwrap();
        if let Some(connection) = connections.get_mut(producer_id) {
            connection
                .consumer_pipelines
                .retain(|p| p.name() != consumer_pipeline.name());
            
            if connection.consumer_pipelines.is_empty() {
                connection.state = ConnectionState::Disconnected;
            }
        }

        Ok(())
    }

    /// Remove a producer and all its consumers
    pub fn remove_producer(&self, producer_id: &str) -> Result<(), InterPipelineError> {
        info!("Removing producer: {}", producer_id);

        let mut connections = self.connections.write().unwrap();
        if let Some(connection) = connections.remove(producer_id) {
            // Stop all consumer pipelines
            for pipeline in &connection.consumer_pipelines {
                let _ = pipeline.set_state(gst::State::Null);
            }

            // Clean up intersink
            if let Some(intersink) = &connection.intersink {
                let _ = intersink.set_state(gst::State::Null);
            }

            info!("Producer {} removed successfully", producer_id);
            Ok(())
        } else {
            Err(InterPipelineError::ProducerNotFound(
                producer_id.to_string(),
            ))
        }
    }

    /// Get connection state for a producer
    pub fn get_connection_state(&self, producer_id: &str) -> Option<ConnectionState> {
        let connections = self.connections.read().unwrap();
        connections.get(producer_id).map(|c| c.state.clone())
    }

    /// List all active producers
    pub fn list_producers(&self) -> Vec<String> {
        let connections = self.connections.read().unwrap();
        connections.keys().cloned().collect()
    }

    /// Get consumer count for a producer
    pub fn get_consumer_count(&self, producer_id: &str) -> usize {
        let connections = self.connections.read().unwrap();
        connections
            .get(producer_id)
            .map(|c| c.consumer_pipelines.len())
            .unwrap_or(0)
    }

    /// Check if producer has any active consumers
    pub fn has_consumers(&self, producer_id: &str) -> bool {
        self.get_consumer_count(producer_id) > 0
    }

    /// Create a simple passthrough consumer for testing
    pub fn create_test_consumer(
        &self,
        producer_id: &str,
    ) -> Result<gst::Pipeline, InterPipelineError> {
        let (pipeline, intersrc) = self.create_consumer_pipeline(producer_id, "test")?;

        // Add a fakesink for testing
        let fakesink = gst::ElementFactory::make("fakesink")
            .name("test-sink")
            .property("sync", false)
            .build()
            .map_err(|_| InterPipelineError::ElementCreation("fakesink".to_string()))?;

        pipeline
            .add(&fakesink)
            .map_err(|_| InterPipelineError::PipelineAddError)?;

        intersrc
            .link(&fakesink)
            .map_err(|_| InterPipelineError::LinkError)?;

        Ok(pipeline)
    }
}

impl Default for InterPipelineManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init() {
        let _ = gst::init();
        let _ = tracing_subscriber::fmt()
            .with_env_filter("stream_manager=debug")
            .try_init();
    }

    #[test]
    fn test_producer_registration() {
        init();

        let manager = InterPipelineManager::new();
        let pipeline = gst::Pipeline::new();

        let intersink = manager
            .register_producer("test-producer", &pipeline)
            .expect("Failed to register producer");

        assert_eq!(intersink.name(), "intersink-test-producer");
        assert!(manager.list_producers().contains(&"test-producer".to_string()));
    }

    #[test]
    fn test_consumer_creation() {
        init();

        let manager = InterPipelineManager::new();
        let producer_pipeline = gst::Pipeline::new();

        // Register producer first
        manager
            .register_producer("test-producer", &producer_pipeline)
            .expect("Failed to register producer");

        // Create consumer
        let (consumer_pipeline, intersrc) = manager
            .create_consumer_pipeline("test-producer", "consumer1")
            .expect("Failed to create consumer");

        assert_eq!(
            consumer_pipeline.name(),
            "consumer-test-producer-consumer1"
        );
        assert_eq!(intersrc.name(), "intersrc-test-producer-consumer1");
        assert_eq!(manager.get_consumer_count("test-producer"), 1);
    }

    #[test]
    fn test_inter_connection() {
        init();

        let manager = InterPipelineManager::new();
        let producer_pipeline = gst::Pipeline::new();

        // Create a test source element
        let source = gst::ElementFactory::make("videotestsrc")
            .name("test-source")
            .property("is-live", true)
            .build()
            .expect("Failed to create videotestsrc");

        producer_pipeline
            .add(&source)
            .expect("Failed to add source to pipeline");

        // Register producer
        let intersink = manager
            .register_producer("test-producer", &producer_pipeline)
            .expect("Failed to register producer");

        // Connect source to intersink
        source.link(&intersink).expect("Failed to link to intersink");

        // Verify connection
        assert_eq!(
            manager.get_connection_state("test-producer"),
            Some(ConnectionState::Connecting)
        );
    }

    #[test]
    fn test_multiple_consumers() {
        init();

        let manager = InterPipelineManager::new();
        let producer_pipeline = gst::Pipeline::new();

        // Register producer
        manager
            .register_producer("test-producer", &producer_pipeline)
            .expect("Failed to register producer");

        // Create multiple consumers
        let (_consumer1, _) = manager
            .create_consumer_pipeline("test-producer", "consumer1")
            .expect("Failed to create consumer1");

        let (_consumer2, _) = manager
            .create_consumer_pipeline("test-producer", "consumer2")
            .expect("Failed to create consumer2");

        let (_consumer3, _) = manager
            .create_consumer_pipeline("test-producer", "consumer3")
            .expect("Failed to create consumer3");

        assert_eq!(manager.get_consumer_count("test-producer"), 3);
        assert!(manager.has_consumers("test-producer"));
    }

    #[test]
    fn test_consumer_disconnection() {
        init();

        let manager = InterPipelineManager::new();
        let producer_pipeline = gst::Pipeline::new();

        // Register producer
        manager
            .register_producer("test-producer", &producer_pipeline)
            .expect("Failed to register producer");

        // Create consumer
        let (consumer_pipeline, _) = manager
            .create_consumer_pipeline("test-producer", "consumer1")
            .expect("Failed to create consumer");

        assert_eq!(manager.get_consumer_count("test-producer"), 1);

        // Disconnect consumer
        manager
            .disconnect_consumer("test-producer", &consumer_pipeline)
            .expect("Failed to disconnect consumer");

        assert_eq!(manager.get_consumer_count("test-producer"), 0);
        assert!(!manager.has_consumers("test-producer"));
    }

    #[test]
    fn test_producer_removal() {
        init();

        let manager = InterPipelineManager::new();
        let producer_pipeline = gst::Pipeline::new();

        // Register producer
        manager
            .register_producer("test-producer", &producer_pipeline)
            .expect("Failed to register producer");

        // Create consumers
        manager
            .create_consumer_pipeline("test-producer", "consumer1")
            .expect("Failed to create consumer");

        assert!(manager.list_producers().contains(&"test-producer".to_string()));

        // Remove producer
        manager
            .remove_producer("test-producer")
            .expect("Failed to remove producer");

        assert!(!manager.list_producers().contains(&"test-producer".to_string()));
    }

    #[test]
    fn test_nonexistent_producer() {
        init();

        let manager = InterPipelineManager::new();

        // Try to create consumer for non-existent producer
        let result = manager.create_consumer_pipeline("nonexistent", "consumer1");
        assert!(result.is_err());

        // Try to remove non-existent producer
        let result = manager.remove_producer("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_connection_state_tracking() {
        init();

        let manager = InterPipelineManager::new();
        let producer_pipeline = gst::Pipeline::new();

        // Initially no state
        assert_eq!(manager.get_connection_state("test-producer"), None);

        // Register producer - should be Connecting
        manager
            .register_producer("test-producer", &producer_pipeline)
            .expect("Failed to register producer");

        assert!(matches!(
            manager.get_connection_state("test-producer"),
            Some(ConnectionState::Connecting)
        ));

        // Create consumer - should be Connected
        manager
            .create_consumer_pipeline("test-producer", "consumer1")
            .expect("Failed to create consumer");

        assert!(matches!(
            manager.get_connection_state("test-producer"),
            Some(ConnectionState::Connected)
        ));
    }

    #[test]
    fn test_test_consumer_creation() {
        init();

        let manager = InterPipelineManager::new();
        let producer_pipeline = gst::Pipeline::new();

        // Register producer
        manager
            .register_producer("test-producer", &producer_pipeline)
            .expect("Failed to register producer");

        // Create test consumer
        let test_pipeline = manager
            .create_test_consumer("test-producer")
            .expect("Failed to create test consumer");

        assert_eq!(test_pipeline.name(), "consumer-test-producer-test");
        assert_eq!(manager.get_consumer_count("test-producer"), 1);
    }
}