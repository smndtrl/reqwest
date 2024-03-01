use hyper::{body::Body, Method, Request as HyperRequest, Response, Uri, Version};
use core::marker::PhantomData;

use spin_sdk::http::conversions::TryIntoOutgoingRequest;
use spin_sdk::http::conversions::TryFromIncomingResponse;
use spin_sdk::http::conversions::TryIntoBody;
use spin_sdk::http::Headers;
use spin_sdk::http::OutgoingRequest;
use spin_sdk::http::Scheme;
use spin_sdk::http::conversions::IntoBody;

use super::{Request as ReqwestRequest, Body as ReqwestBody};

impl IntoBody for ReqwestBody {
    fn into_body(self) -> Vec<u8> { 
        self.as_bytes().map(|v| v.to_vec()).expect("we can't do streaming")
     }

}

impl TryIntoOutgoingRequest for ReqwestRequest {
    type Error = anyhow::Error;

    fn try_into_outgoing_request(self) -> Result<(spin_sdk::http::OutgoingRequest, std::option::Option<Vec<u8>>), Self::Error> {
        let headers = self
            .headers()
            .into_iter()
            .map(|(n, v)| (n.as_str().to_owned(), v.as_bytes().to_owned()))
            .collect::<Vec<_>>();
        let request = OutgoingRequest::new(Headers::new());
        request
            .set_method(&self.method().clone().into())
            // .map_err(|()| crate::error::builder("error setting method"))?;
            .map_err(|()| {
                anyhow::anyhow!(
                    "error setting method to {}",
                    Method::from(self.method().clone())
                )
            })?;
        request
            .set_path_with_query(Some(&self.url()[url::Position::BeforePath..]))
            // .map_err(|()| crate::error::builder("error setting path"))?;
            .map_err(|()| {
                anyhow::anyhow!("error setting path to {:?}", &self.url()[url::Position::BeforePath..])
            })?;
        let scheme: Scheme = match self.url().scheme() {
            "http" => Scheme::Http,
            "https" => Scheme::Https,
            s => Scheme::Other(s.to_owned()),
        };
        request
            .set_scheme(Some(&scheme))
            .map_err(|()| crate::error::builder("error setting scheme"))?;
            // .map_err(|()| anyhow::anyhow!("error setting scheme to {scheme:?}"))?;
        request
            .set_authority(self.url().host_str())
            .map_err(|()| crate::error::builder("error setting authority"))?;
            // .map_err(|()| {
            //     anyhow::anyhow!("error setting authority to {:?}", self.uri().authority())
            // })?;
        // let buffer = TryIntoBody::try_into_body(self.into_body())
        // .map_err(crate::error::body)?;
        println!("converted to OutgoingRequest");
        Ok((request, None))
        // Ok((request, Some(buffer)))
    }
}


use std::fmt;

#[derive(Debug)]
pub struct MyError {
    message: String,
}

impl MyError {
    fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for MyError {}
use spin_sdk::http::SendError;
use super::executor;
use futures::SinkExt;
use spin_sdk::wit::wasi::io::streams::{StreamError};
use futures_util::Sink;
use spin_sdk::http::OutgoingBody;
use spin_sdk::wit::wasi::io::streams::OutputStream;
use std::sync::Arc;
use spin_sdk::wit::wasi::io;
use futures_util::sink;
use futures_util::future;

use std::task::Waker;
use std::sync::Mutex;
use std::cell::RefCell;
use std::sync::RwLock;
use std::borrow::Borrow;
pub struct Client {

}

static WAKERS: Mutex<Vec<(io::poll::Pollable, Waker)>> = Mutex::new(Vec::new());

pub(crate) fn outgoing_body_sink(body: OutgoingBody) -> impl Sink<Vec<u8>, Error = StreamError> {
    struct Outgoing(Option<(OutputStream, OutgoingBody)>);

    impl Drop for Outgoing {
        fn drop(&mut self) {
            if let Some((stream, body)) = self.0.take() {
                drop(stream);
                _ = OutgoingBody::finish(body, None);
            }
        }
    }

    let stream = body.write().expect("response body should be writable");
    let pair = Arc::new(RwLock::new(Outgoing(Some((stream, body)))));

    sink::unfold((), {
        move |(), chunk: Vec<u8>| {
            future::poll_fn({
                let mut offset = 0;
                let mut flushing = false;
                let pair = pair.clone();

                move |context| {
                    let pair = pair.read().unwrap();
                    let (stream, _) = &pair.0.as_ref().unwrap();
                    loop {
                        match stream.check_write() {
                            Ok(0) => {
                                WAKERS
                                    .lock()
                                    .unwrap()
                                    .push((stream.subscribe(), context.waker().clone()));

                                break Poll::Pending;
                            }
                            Ok(count) => {
                                if offset == chunk.len() {
                                    if flushing {
                                        break Poll::Ready(Ok(()));
                                    } else {
                                        match stream.flush() {
                                            Ok(()) => flushing = true,
                                            Err(StreamError::Closed) => break Poll::Ready(Ok(())),
                                            Err(e) => break Poll::Ready(Err(e)),
                                        }
                                    }
                                } else {
                                    let count =
                                        usize::try_from(count).unwrap().min(chunk.len() - offset);

                                    match stream.write(&chunk[offset..][..count]) {
                                        Ok(()) => {
                                            offset += count;
                                        }
                                        Err(e) => break Poll::Ready(Err(e)),
                                    }
                                }
                            }
                            // If the stream is closed but the entire chunk was
                            // written then we've done all we could so this
                            // chunk is now complete.
                            Err(StreamError::Closed) if offset == chunk.len() => {
                                break Poll::Ready(Ok(()))
                            }
                            Err(e) => break Poll::Ready(Err(e)),
                        }
                    }
                }
            })
        }
    })
}

impl Client
// where 
// B: IntoBody,
// B: Body + Send + 'static + Unpin,
// B::Data: Send,
// B::Error: Into<Box<dyn StdError + Send + Sync>>,
 {

    pub fn new() -> Self {
        Self {}
    }

    pub fn request<I, O>(
        &self, request: I
    ) 
    -> WasiHttpResponseFuture<O>
    where
        I: TryIntoOutgoingRequest + Send + std::fmt::Debug +  'static,
        I::Error: Into<Box<dyn std::error::Error + Send + Sync>> + 'static,
        O: TryFromIncomingResponse,
        O::Error: Into<Box<dyn std::error::Error + Send + Sync>> + 'static,
    {
        println!("request {:?}", request);

        // let test = Box::pin(async move {
        //     let response = spin_sdk::http::send(req);
        //     response
        // });

        // let response = spin_sdk::http::send(req).await
        //     .map_err(|| );
        // WasiHttpResponseFuture::new(spin_sdk::http::send(req))
        use std::io::Write;
        WasiHttpResponseFuture::new(async move {
            let (request, body_buffer) = I::try_into_outgoing_request(request)
            .map_err(|e| SendError::RequestConversion(e.into()))?;

        println!("{:?} {:?}", request, body_buffer);



            let response = if let Some(body_buffer) = body_buffer {
                let outgoing_body = request
                .body()
                .map_err(|e| panic!("outgoing request write failed"))?;

                // let stream  = std::sync::Arc::new(outgoing_body.write().unwrap());
                

                let mut body_sink = outgoing_body_sink(outgoing_body);
                body_sink.send(body_buffer).await.map_err(SendError::Io)?;
                drop(body_sink);


            // };


            println!("fake body outgoing");
            executor::outgoing_request_send(request)
                    .await
                    .map_err(SendError::Http)?
            // todo!("no body yet")
            } else {
                println!("no body outgoing");
                executor::outgoing_request_send(request)
                    .await
                    .map_err(SendError::Http)?
            };

            let d = TryFromIncomingResponse::try_from_incoming_response(response)
            .await
            .map_err(|e: O::Error| SendError::ResponseConversion(e.into()));
d
            // Ok(SpinResponse::new(202, vec![]))
        })
    }
}

use std::future::Future;
use std::task::Poll;
use spin_sdk::http::{Response as SpinResponse, Request as SpinRequest};

impl<O> WasiHttpResponseFuture<O> {
    fn new<F>(value: F) -> Self
    where
        F: Future<Output = Result<O, spin_sdk::http::SendError>> + Send + 'static,
        // I: TryIntoOutgoingRequest,
        // I::Error: Into<Box<dyn std::error::Error + Send + Sync>> + 'static,
        O: TryFromIncomingResponse,
        O::Error: Into<Box<dyn std::error::Error + Send + Sync>> + 'static,
    {
        Self {
            inner: Box::pin(value),
        }
    }
}

// impl fmt::Debug for WasiHttpResponseFuture {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         f.pad("Future<Response>")
//     }
// }

impl<O> Future for WasiHttpResponseFuture<O> {
    type Output = Result<O, spin_sdk::http::SendError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        self.inner.as_mut().poll(cx)
    }
}


#[must_use = "futures do nothing unless polled"]
pub struct WasiHttpResponseFuture<O> {
    inner: 
        Pin<Box<dyn Future<Output = Result<O, spin_sdk::http::SendError>> + Send>>,
    
}

// impl WasiHttpResponseFuture {
//     fn new<T>(value: T) -> Self
//     where
//     //     T: IntoBody
//         // F: T
//         T: Future<Output = Result<SpinResponse, spin_sdk::http::SendError>> + Send + 'static,
//     {
//         Self {
//             inner: Box::pin(value),
//         }
//     }
// }

// impl fmt::Debug for WasiHttpResponseFuture {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         f.pad("Future<Response>")
//     }
// }

// impl Future for WasiHttpResponseFuture {
//     type Output = Result<SpinResponse, crate::Error>;

//     fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        
//         let future_result = spin_sdk::http::send(self.inner);

//         // spin_sdk::http::send(self.inner)

//         let mut async_function = Box::pin(future_result);

//         // let poll_result = future_result.as_mut().poll(cx);  
        
//         // Poll the async function
//         match async_function.as_mut().poll(cx) {
//             Poll::Pending => {
//                 // The async function is not ready yet
//                 println!("Async function is pending");
//                 Poll::Pending
//             }
//             Poll::Ready(Ok(d)) => {
//                 Poll::Ready(Ok(d))
//             }
//             // Poll::Ready(d) => {
//             //     // The async function has completed
//             //     println!("Async function is ready");
//             //     println!("Async function is ready {:?}", d);
//             //     Poll::Ready(d.expect("REASON"))
//             //     // Poll::Pending
//             // }
//         }
//     }
// }


// #[must_use = "futures do nothing unless polled"]
// pub struct WasiHttpResponseFuture {
//     inner: Pin<Box<dyn Future<Output = Result<SpinResponse, spin_sdk::http::SendError>> + Send>>,
    
// }

use std::pin::Pin;
use log::warn;



    use http_body_util::combinators::BoxBody;
    use crate::wasi::async_impl::body::ResponseBody;
pub type InnerBody = BoxBody<hyper::body::Bytes, crate::Error>;


// use crate::wasi::async_impl::executor::outgoing_request_send;
// use crate::wasi::async_impl::conversions::*;
// use crate::wasi::wit::wasi::http::*;
// use crate::wasi::wit::wasi::io::*;
// use crate::wasi::wit::wasi::io::streams::{ StreamError};
// use crate::wasi::wit::wasi::http::types::*;
// use super::executor;

//
// impl OutgoingRequest {
//     /// Construct a `Sink` which writes chunks to the body of the specified response.
//     ///
//     /// # Panics
//     ///
//     /// Panics if the body was already taken.
//     pub fn take_body(&self) -> impl futures::Sink<Vec<u8>, Error = StreamError> {
//         executor::outgoing_body(self.body().expect("request body was already taken"))
//     }
// }

// impl IncomingResponse {
//     /// Return a `Stream` from which the body of the specified response may be read.
//     ///
//     /// # Panics
//     ///
//     /// Panics if the body was already consumed.
//     // TODO: This should ideally take ownership of `self` and be called `into_body_stream` (i.e. symmetric with
//     // `IncomingRequest::into_body_stream`).  However, as of this writing, `wasmtime-wasi-http` is implemented in
//     // such a way that dropping an `IncomingResponse` will cause the request to be cancelled, meaning the caller
//     // won't necessarily have a chance to send the request body if they haven't started doing so yet (or, if they
//     // have started, they might not be able to finish before the connection is closed).  See
//     // https://github.com/bytecodealliance/wasmtime/issues/7413 for details.
//     pub fn take_body_stream(&self) -> impl futures::Stream<Item = Result<Vec<u8>, streams::Error>> {
//         executor::incoming_body(self.consume().expect("response body was already consumed"))
//     }

//     /// Return a `Vec<u8>` of the body or fails
//     ///
//     /// # Panics
//     ///
//     /// Panics if the body was already consumed.
//     pub async fn into_body(self) -> Result<Vec<u8>, streams::Error> {
//         use futures::TryStreamExt;
//         let mut stream = self.take_body_stream();
//         let mut body = Vec::new();
//         while let Some(chunk) = stream.try_next().await? {
//             body.extend(chunk);
//         }
//         Ok(body)
//     }
// }

