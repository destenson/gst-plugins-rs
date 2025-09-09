use opentelemetry::{
    global,
    trace::{
        Span, SpanKind, Status, TraceContextExt, Tracer,
    },
    Context, KeyValue,
};
use std::future::Future;
use tracing::{debug, instrument};

pub struct SpanManager {}

impl SpanManager {
    pub fn new() -> Self {
        Self {}
    }
    
    #[instrument(skip(self))]
    pub fn create_stream_span(&self, stream_id: &str, uri: &str) -> Context {
        let tracer = global::tracer("stream-manager");
        let mut span = tracer
            .span_builder("stream.process")
            .with_kind(SpanKind::Internal)
            .with_attributes(vec![
                KeyValue::new("stream.id", stream_id.to_string()),
                KeyValue::new("stream.uri", uri.to_string()),
            ])
            .start(&tracer);
            
        span.set_status(Status::Ok);
        
        let cx = Context::current();
        cx.with_span(span)
    }
    
    #[instrument(skip(self))]
    pub fn create_pipeline_span(&self, pipeline_id: &str, operation: &str) -> Context {
        let tracer = global::tracer("stream-manager");
        let mut span = tracer
            .span_builder(format!("pipeline.{}", operation))
            .with_kind(SpanKind::Internal)
            .with_attributes(vec![
                KeyValue::new("pipeline.id", pipeline_id.to_string()),
                KeyValue::new("pipeline.operation", operation.to_string()),
            ])
            .start(&tracer);
            
        span.set_status(Status::Ok);
        
        let cx = Context::current();
        cx.with_span(span)
    }
    
    #[instrument(skip(self))]
    pub fn create_recording_span(&self, stream_id: &str, file_path: &str) -> Context {
        let tracer = global::tracer("stream-manager");
        let mut span = tracer
            .span_builder("recording.write")
            .with_kind(SpanKind::Internal)
            .with_attributes(vec![
                KeyValue::new("stream.id", stream_id.to_string()),
                KeyValue::new("recording.path", file_path.to_string()),
            ])
            .start(&tracer);
            
        span.set_status(Status::Ok);
        
        let cx = Context::current();
        cx.with_span(span)
    }
    
    #[instrument(skip(self))]
    pub fn create_api_span(&self, method: &str, path: &str) -> Context {
        let tracer = global::tracer("stream-manager");
        let mut span = tracer
            .span_builder(format!("api.{}", method.to_lowercase()))
            .with_kind(SpanKind::Server)
            .with_attributes(vec![
                KeyValue::new("http.method", method.to_string()),
                KeyValue::new("http.path", path.to_string()),
            ])
            .start(&tracer);
            
        span.set_status(Status::Ok);
        
        let cx = Context::current();
        cx.with_span(span)
    }
    
    #[instrument(skip(self))]
    pub fn create_inference_span(&self, model_name: &str, batch_size: usize) -> Context {
        let tracer = global::tracer("stream-manager");
        let mut span = tracer
            .span_builder("inference.process")
            .with_kind(SpanKind::Internal)
            .with_attributes(vec![
                KeyValue::new("inference.model", model_name.to_string()),
                KeyValue::new("inference.batch_size", batch_size as i64),
            ])
            .start(&tracer);
            
        span.set_status(Status::Ok);
        
        let cx = Context::current();
        cx.with_span(span)
    }
    
    #[instrument(skip(self))]
    pub fn create_storage_span(&self, operation: &str, path: &str) -> Context {
        let tracer = global::tracer("stream-manager");
        let mut span = tracer
            .span_builder(format!("storage.{}", operation))
            .with_kind(SpanKind::Internal)
            .with_attributes(vec![
                KeyValue::new("storage.operation", operation.to_string()),
                KeyValue::new("storage.path", path.to_string()),
            ])
            .start(&tracer);
            
        span.set_status(Status::Ok);
        
        let cx = Context::current();
        cx.with_span(span)
    }
    
    pub async fn with_span<F, T>(&self, context: Context, future: F) -> T
    where
        F: Future<Output = T>,
    {
        // Execute the future within the given context
        let _guard = context.clone().attach();
        future.await
    }
    
    pub fn add_event(&self, context: &Context, name: &str, attributes: Vec<KeyValue>) {
        let span = context.span();
        let span_context = span.span_context();
        if span_context.is_valid() {
            debug!(
                trace_id = %span_context.trace_id(),
                event_name = %name,
                "Adding event to span"
            );
        }
        
        span.add_event(name.to_string(), attributes);
    }
    
    pub fn set_error(&self, context: &Context, error: &str) {
        context.span().set_status(Status::error(error.to_string()));
        context.span().add_event(
            "error",
            vec![KeyValue::new("error.message", error.to_string())],
        );
    }
    
    pub fn add_attributes(&self, context: &Context, attributes: Vec<KeyValue>) {
        context.span().set_attributes(attributes);
    }
}

pub trait TracedOperation {
    fn trace_operation<'a>(
        &'a self,
        span_manager: &'a SpanManager,
        operation: &'a str,
    ) -> TracedOperationGuard<'a>;
}

pub struct TracedOperationGuard<'a> {
    span_manager: &'a SpanManager,
    context: Context,
    operation: &'a str,
}

impl<'a> TracedOperationGuard<'a> {
    pub fn new(span_manager: &'a SpanManager, context: Context, operation: &'a str) -> Self {
        Self {
            span_manager,
            context,
            operation,
        }
    }
    
    pub fn add_event(&self, name: &str, attributes: Vec<KeyValue>) {
        self.span_manager.add_event(&self.context, name, attributes);
    }
    
    pub fn set_error(&self, error: &str) {
        self.span_manager.set_error(&self.context, error);
    }
    
    pub fn add_attributes(&self, attributes: Vec<KeyValue>) {
        self.span_manager.add_attributes(&self.context, attributes);
    }
}

impl<'a> Drop for TracedOperationGuard<'a> {
    fn drop(&mut self) {
        debug!("Completed traced operation: {}", self.operation);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_span_creation() {
        let span_manager = SpanManager::new();
        
        let context = span_manager.create_stream_span("test-stream", "rtsp://test");
        assert!(context.has_active_span());
    }
    
    #[tokio::test]
    async fn test_span_with_attributes() {
        let span_manager = SpanManager::new();
        
        let context = span_manager.create_pipeline_span("test-pipeline", "start");
        span_manager.add_attributes(
            &context,
            vec![
                KeyValue::new("test.attribute", "value"),
                KeyValue::new("test.number", 42i64),
            ],
        );
        
        assert!(context.has_active_span());
    }
    
    #[tokio::test]
    async fn test_span_error_handling() {
        let span_manager = SpanManager::new();
        
        let context = span_manager.create_api_span("GET", "/test");
        span_manager.set_error(&context, "Test error");
        
        assert!(context.has_active_span());
    }
}