use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use futures::future::LocalBoxFuture;
use std::{
    future::{ready, Ready},
    rc::Rc,
};
use tracing::{info, warn, error, debug};
use uuid::Uuid;
use std::time::Instant;

pub fn error_handler() -> ErrorHandler {
    ErrorHandler
}

pub fn request_logger() -> RequestLogger {
    RequestLogger
}

pub struct ErrorHandler;

impl<S, B> Transform<S, ServiceRequest> for ErrorHandler
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = ErrorHandlerMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(ErrorHandlerMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct ErrorHandlerMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for ErrorHandlerMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();

        Box::pin(async move {
            let result = service.call(req).await;
            
            if let Err(ref err) = result {
                error!("Request error: {:?}", err);
            }
            
            result
        })
    }
}

pub struct RequestLogger;

impl<S, B> Transform<S, ServiceRequest> for RequestLogger
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = RequestLoggerMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RequestLoggerMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct RequestLoggerMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for RequestLoggerMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        let request_id = Uuid::new_v4();
        let method = req.method().clone();
        let path = req.path().to_string();
        let remote_addr = req.peer_addr();
        let start_time = Instant::now();

        // Add request ID to request extensions
        req.extensions_mut().insert(request_id);

        debug!(
            request_id = %request_id,
            method = %method,
            path = %path,
            remote_addr = ?remote_addr,
            "Incoming request"
        );

        Box::pin(async move {
            let result = service.call(req).await;
            let elapsed = start_time.elapsed();
            
            match &result {
                Ok(res) => {
                    let status = res.status();
                    if status.is_success() {
                        info!(
                            request_id = %request_id,
                            method = %method,
                            path = %path,
                            status = %status,
                            duration_ms = %elapsed.as_millis(),
                            "Request completed"
                        );
                    } else if status.as_u16() == 101 {
                        // WebSocket upgrade - not an error
                        debug!(
                            request_id = %request_id,
                            method = %method,
                            path = %path,
                            status = %status,
                            duration_ms = %elapsed.as_millis(),
                            "WebSocket upgrade"
                        );
                    } else if status.is_client_error() {
                        warn!(
                            request_id = %request_id,
                            method = %method,
                            path = %path,
                            status = %status,
                            duration_ms = %elapsed.as_millis(),
                            "Client error"
                        );
                    } else {
                        error!(
                            request_id = %request_id,
                            method = %method,
                            path = %path,
                            status = %status,
                            duration_ms = %elapsed.as_millis(),
                            "Server error"
                        );
                    }
                }
                Err(err) => {
                    error!(
                        request_id = %request_id,
                        method = %method,
                        path = %path,
                        error = %err,
                        duration_ms = %elapsed.as_millis(),
                        "Request failed"
                    );
                }
            }
            
            result
        })
    }
}

pub struct AuthMiddleware;

impl<S, B> Transform<S, ServiceRequest> for AuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthMiddlewareService<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthMiddlewareService {
            service: Rc::new(service),
        }))
    }
}

pub struct AuthMiddlewareService<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for AuthMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();
        
        Box::pin(async move {
            // TODO: Implement actual authentication logic
            // For now, just pass through
            service.call(req).await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, App, HttpResponse};

    async fn test_handler() -> HttpResponse {
        HttpResponse::Ok().body("test")
    }

    #[actix_web::test]
    async fn test_request_logger_middleware() {
        let app = test::init_service(
            App::new()
                .wrap(request_logger())
                .route("/test", web::get().to(test_handler))
        ).await;

        let req = test::TestRequest::get()
            .uri("/test")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    #[actix_web::test]
    async fn test_error_handler_middleware() {
        let app = test::init_service(
            App::new()
                .wrap(error_handler())
                .route("/test", web::get().to(test_handler))
        ).await;

        let req = test::TestRequest::get()
            .uri("/test")
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }
}