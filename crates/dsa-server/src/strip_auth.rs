use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error,
};
use futures::future::{ok, LocalBoxFuture, Ready};
use std::rc::Rc;

pub struct StripAuth;

impl<S, B> Transform<S, ServiceRequest> for StripAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = StripAuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(StripAuthMiddleware {
            service: Rc::new(service),
        })
    }
}

pub struct StripAuthMiddleware<S> {
    service: Rc<S>,
}

static LOCAL_MODE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

pub fn set_local_mode(is_local: bool) {
    LOCAL_MODE.store(is_local, std::sync::atomic::Ordering::Relaxed);
}

impl<S, B> Service<ServiceRequest> for StripAuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        if LOCAL_MODE.load(std::sync::atomic::Ordering::Relaxed) {
            if req.headers().contains_key("Authorization") {
                req.headers_mut().remove("Authorization");
            }
        }

        let svc = self.service.clone();
        Box::pin(async move { svc.call(req).await })
    }
}
