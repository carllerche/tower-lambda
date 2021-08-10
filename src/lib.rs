use bytes::buf::Buf;
use http::{Request, Response};
use tower_service::Service;

use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;

pub type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

pub async fn run<S, B>(service: S) -> Result<(), Error>
where
    S: Service<Request<hyper::Body>, Response = Response<B>> + Send + Clone + 'static,
    S::Error: Into<Error>,
    S::Future: Send + 'static,
    B: hyper::body::HttpBody + Send + 'static,
    B::Data: Send,
    B::Error: Into<Error>,
{
    let handler = lambda_http::handler(ServiceHandler(service, PhantomData));
    lambda_runtime::run(handler).await
}

struct ServiceHandler<S, B>(S, PhantomData<B>);

impl<S, B> lambda_http::Handler<'static> for ServiceHandler<S, B>
where
    S: Service<Request<hyper::Body>, Response = Response<B>> + Send + Clone + 'static,
    S::Error: Into<Error>,
    S::Future: Send + 'static,
    B: hyper::body::HttpBody + Send + 'static,
    B::Data: Send,
    B::Error: Into<Error>,
{
    type Error = Error;
    type Response = Response<Vec<u8>>;
    type Fut = Pin<Box<dyn Future<Output = Result<Self::Response, Error>> + Send>>;

    fn call(&self, request: lambda_http::Request, _context: lambda_http::Context) -> Self::Fut {
        let mut service = self.0.clone();

        Box::pin(async move {
            // Map body types
            let request = request.map(|body| body.as_ref().to_vec().into());

            // Call the Tower service
            let response = match service.call(request).await {
                Ok(response) => response,
                Err(e) => return Err(e.into()),
            };

            let (head, body) = response.into_parts();

            let mut body = hyper::body::aggregate(body).await.map_err(Into::into)?;
            let body = body.copy_to_bytes(body.remaining());

            // Ok(response)
            Ok(Response::from_parts(head, body.to_vec()))
        })
    }
}
