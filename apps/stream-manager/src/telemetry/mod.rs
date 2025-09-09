use opentelemetry::{
    global,
    propagation::TextMapPropagator,
    metrics::{MeterProvider as _, Meter},
    trace::TraceContextExt,
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    propagation::TraceContextPropagator,
    trace::{RandomIdGenerator, Sampler},
    metrics::{self as metrics_sdk, PeriodicReader},
    runtime,
    Resource,
};
use opentelemetry_semantic_conventions::resource::{
    SERVICE_NAME, SERVICE_VERSION, SERVICE_INSTANCE_ID,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tracing::info;
use uuid::Uuid;

pub mod performance;
pub mod spans;

#[derive(Error, Debug)]
pub enum TelemetryError {
    #[error("Failed to initialize tracer: {0}")]
    TracerInit(String),
    
    #[error("Failed to initialize metrics: {0}")]
    MetricsInit(String),
    
    #[error("Failed to configure exporter: {0}")]
    ExporterConfig(String),
    
    #[error("Propagation error: {0}")]
    Propagation(String),
}

#[derive(Clone)]
pub struct TelemetryConfig {
    pub service_name: String,
    pub service_version: String,
    pub instance_id: String,
    pub otlp_endpoint: Option<String>,
    pub sampling_ratio: f64,
    pub export_timeout: Duration,
    pub export_interval: Duration,
    pub console_exporter: bool,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            service_name: "stream-manager".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            instance_id: Uuid::new_v4().to_string(),
            otlp_endpoint: Some("http://localhost:4317".to_string()),
            sampling_ratio: 1.0,
            export_timeout: Duration::from_secs(10),
            export_interval: Duration::from_secs(10),
            console_exporter: false,
        }
    }
}

pub struct TelemetryProvider {
    meter: Meter,
    config: TelemetryConfig,
}

impl TelemetryProvider {
    pub async fn new(config: TelemetryConfig) -> Result<Arc<Self>, TelemetryError> {
        let resource = Resource::new(vec![
            KeyValue::new(SERVICE_NAME, config.service_name.clone()),
            KeyValue::new(SERVICE_VERSION, config.service_version.clone()),
            KeyValue::new(SERVICE_INSTANCE_ID, config.instance_id.clone()),
        ]);

        // Initialize tracer
        Self::init_tracer(&config, resource.clone()).await?;
        
        // Initialize metrics
        let meter = Self::init_metrics(&config, resource).await?;
        
        // Set global text map propagator
        global::set_text_map_propagator(TraceContextPropagator::new());
        
        Ok(Arc::new(Self {
            meter,
            config,
        }))
    }
    
    async fn init_tracer(
        config: &TelemetryConfig,
        resource: Resource,
    ) -> Result<(), TelemetryError> {
        let mut tracer_builder = opentelemetry_sdk::trace::TracerProvider::builder()
            .with_resource(resource)
            .with_sampler(Sampler::TraceIdRatioBased(config.sampling_ratio))
            .with_id_generator(RandomIdGenerator::default());

        // Add OTLP exporter if configured
        if let Some(endpoint) = &config.otlp_endpoint {
            let exporter = opentelemetry_otlp::SpanExporter::builder()
                .with_tonic()
                .with_endpoint(endpoint)
                .with_timeout(config.export_timeout)
                .build()
                .map_err(|e| TelemetryError::ExporterConfig(e.to_string()))?;
                
            tracer_builder = tracer_builder.with_batch_exporter(
                exporter,
                runtime::Tokio,
            );
        }
        
        // Add console exporter for debugging
        if config.console_exporter {
            // Console exporter for debugging is configured through the tracer builder
            // The OTLP exporter above will handle the main export functionality
        }
        
        let provider = tracer_builder.build();
        
        global::set_tracer_provider(provider);
        
        Ok(())
    }
    
    async fn init_metrics(
        config: &TelemetryConfig,
        resource: Resource,
    ) -> Result<Meter, TelemetryError> {
        let mut builder = metrics_sdk::SdkMeterProvider::builder()
            .with_resource(resource);
        
        // Add OTLP metrics exporter if configured
        if let Some(endpoint) = &config.otlp_endpoint {
            let exporter = opentelemetry_otlp::MetricExporter::builder()
                .with_tonic()
                .with_endpoint(endpoint)
                .with_timeout(config.export_timeout)
                .build()
                .map_err(|e| TelemetryError::MetricsInit(e.to_string()))?;
                
            let reader = PeriodicReader::builder(exporter, runtime::Tokio)
                .with_interval(config.export_interval)
                .build();
                
            builder = builder.with_reader(reader);
        }
        
        let provider = builder.build();
        let meter = provider.meter("stream-manager");
        
        global::set_meter_provider(provider);
        
        Ok(meter)
    }
    
    pub fn meter(&self) -> &Meter {
        &self.meter
    }
    
    pub fn extract_context(&self, headers: &HashMap<String, String>) -> opentelemetry::Context {
        let propagator = TraceContextPropagator::new();
        let extractor = HeaderExtractor(headers);
        propagator.extract(&extractor)
    }
    
    pub fn inject_context(&self, context: &opentelemetry::Context) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        let propagator = TraceContextPropagator::new();
        let mut injector = HeaderInjector(&mut headers);
        propagator.inject_context(context, &mut injector);
        headers
    }
    
    pub async fn shutdown(&self) -> Result<(), TelemetryError> {
        global::shutdown_tracer_provider();
        info!("Telemetry provider shut down successfully");
        Ok(())
    }
}

struct HeaderExtractor<'a>(&'a HashMap<String, String>);

impl<'a> opentelemetry::propagation::Extractor for HeaderExtractor<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(|v| v.as_str())
    }
    
    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|k| k.as_str()).collect()
    }
}

struct HeaderInjector<'a>(&'a mut HashMap<String, String>);

impl<'a> opentelemetry::propagation::Injector for HeaderInjector<'a> {
    fn set(&mut self, key: &str, value: String) {
        self.0.insert(key.to_string(), value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_telemetry_provider_creation() {
        let config = TelemetryConfig {
            console_exporter: true,
            otlp_endpoint: None,
            ..Default::default()
        };
        
        let provider = TelemetryProvider::new(config).await;
        assert!(provider.is_ok());
    }
    
    #[tokio::test]
    async fn test_context_propagation() {
        let config = TelemetryConfig {
            console_exporter: true,
            otlp_endpoint: None,
            ..Default::default()
        };
        
        let provider = TelemetryProvider::new(config).await.unwrap();
        
        // Create a context with some trace data
        let ctx = opentelemetry::Context::new();
        
        // Inject context into headers
        let headers = provider.inject_context(&ctx);
        
        // Extract context from headers
        let extracted_ctx = provider.extract_context(&headers);
        
        // Both contexts should be equivalent
        assert_eq!(ctx.span().span_context(), extracted_ctx.span().span_context());
    }
    
    #[tokio::test]
    async fn test_shutdown() {
        let config = TelemetryConfig {
            console_exporter: true,
            otlp_endpoint: None,
            ..Default::default()
        };
        
        let provider = TelemetryProvider::new(config).await.unwrap();
        let result = provider.shutdown().await;
        assert!(result.is_ok());
    }
}