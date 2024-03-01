use core::marker::PhantomData;
use std::error::Error as StdError;

use std::future::Future;
use hyper::{body::Body, Method, Request, Response, Uri, Version};

use std::fmt;
use http_body_util::combinators::BoxBody;

use crate::wasi::wit::wasi::http::*;
use crate::wasi::wit::wasi::io::*;
use crate::wasi::wit::wasi::io::streams::{ StreamError};
use crate::wasi::wit::wasi::http::types::*;
use futures::{future, sink, stream, Sink, Stream};
use std::cell::RefCell;
use std::mem;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};
use crate::wasi::async_impl::executor::outgoing_request_send;
// use crate::wasi::async_impl::conversions::*;

pub struct HyperShim<B> {
    data: PhantomData<B>,
}




    // fn run<F>(&self, req: hyper::Request<B>) -> F where 
    //     F: Future<Output = Result<Response<hyper::body::Incoming>, Error>> + Send + 'static {

    //         let headers = req.headers(); //
    //         let mut header_key_values: Vec<(String, Vec<u8>)> = vec![];
    //         for (name, value) in headers.iter() {
    //             match value.to_str() {
    //                 Ok(value) => {
    //                     header_key_values.push((name.as_str().to_string(), value.as_bytes().to_vec()))
    //                 }
    //                 Err(_) => {}
    //             }
    //         }
    
    //         // let (method, url, headers, body, timeout, _version) = request.pieces();
    //         // for (name, value) in headers.iter() {
    //         //     match value.to_str() {
    //         //         Ok(value) => {
    //         //             header_key_values.push((name.as_str().to_string(), value.as_bytes().to_vec()))
    //         //         }
    //         //         Err(_) => {}
    //         //     }
    //         // }

    //         let url =  req.uri();
    //         let method = req.method();
    //         let body = req.body();
    //         // let timeout = req.timeout();
    
    //         let scheme = match url.scheme_str() {
    //             Some("http") => types::Scheme::Http,
    //             Some("https") => types::Scheme::Https,
    //             Some(other) => types::Scheme::Other(other.to_string()),
    //             None => todo!("unknown scheme")
    //         };
    //         let headers = types::Fields::from_list(&header_key_values);
    //         let path_with_query = match url.query() {
    //             Some(query) => format!("{}?{}", url.path(), query),
    //             None => url.path().to_string(),
    //         };
    //         // let request = types::OutgoingRequest::new(
    //         //     &method.into(),
    //         //     Some(&path_with_query),
    //         //     Some(&scheme),
    //         //     Some(url.authority()),
    //         //     &headers,
    //         // );
    
    //         let request = types::OutgoingRequest::new(headers.expect("header error"));
    //         let _ = request.set_method(&method.into());
    //         let _ = request.set_path_with_query(Some(&path_with_query));
    //         let _ = request.set_scheme(Some(&scheme));
    //         let _ = request.set_authority(Some(url.authority().unwrap().as_str()));
    
    
    //         // match body {
    //         //     Some(body) => {
    //         //         let request_body_stream = request.write().map_err(|e| failure_point("outgoing_request_write", e))?;
    //         //         body.write(|chunk| {
    //         //             streams::write(request_body_stream, chunk).map_err(|e| failure_point("write chunk", e))?;
    //         //             Ok(())
    //         //         })?;
    //         //         types::finish_outgoing_stream(request_body_stream, None);
    //         //         streams::drop_output_stream(request_body_stream);
    //         //     }
    //         //     None => {}
    //         // }
    
    //         let outgoing_body = request
    //             .body()
    //             .map_err(|e| failure_point("outgoing request write failed", e)).unwrap();
    
    //         // if let Some(body) = body {
    //             let request_body = outgoing_body
    //                 .write()
    //                 .map_err(|e| failure_point("outgoing request write failed", e)).unwrap();
    
    //             let pollable = request_body.subscribe();
    //             // while !buf.is_empty() {
    //             body.write(|buf| {
    //                 poll::poll(&[&pollable]);
    
    //                 let permit = match request_body.check_write() {
    //                     Ok(n) => n,
    //                     Err(e) => panic!("output stream error: {:?}", e),
    //                 };
    
    //                 let len = buf.len().min(permit as usize);
    //                 let (chunk, _rest) = buf.split_at(len);
    //                 // buf = rest;
    
    //                 match request_body.write(chunk) {
    //                     Err(_) => panic!("output stream error"),
    //                     _ => Ok(()),
    //                 }
    //             })?;
    //             // };
    
    //             match request_body.flush() {
    //                 Err(_) => panic!("output stream error"),
    //                 _ => {}
    //             }
    
    //             poll::poll(&[&pollable]);
    
    //             match request_body.check_write() {
    //                 Ok(_) => {}
    //                 Err(_) => panic!("output stream error"),
    //             };
    //         // }
    
    //         // let future_incoming_response = outgoing_handler::handle(
    //         //     request,
    //         //     Some(types::RequestOptions {
    //         //         connect_timeout_ms: self.inner.connect_timeout.map(|d| d.as_millis() as u32),
    //         //         first_byte_timeout_ms: timeout
    //         //             .or(self.inner.first_byte_timeout)
    //         //             .map(|d| d.as_millis() as u32),
    //         //         between_bytes_timeout_ms: timeout
    //         //             .or(self.inner.between_bytes_timeout)
    //         //             .map(|d| d.as_millis() as u32),
    //         //     }),
    //         // );
    
    //         // let receive_timeout = timeout.or(self.inner.first_byte_timeout);
    //         // let incoming_response =
    //         //     Self::get_incoming_response(future_incoming_response, receive_timeout)?;
    
    //         // let status = types::incoming_response_status(incoming_response);
    //         // let status_code = StatusCode::from_u16(status)
    //         //     .map_err(|e| crate::Error::new(crate::error::Kind::Decode, Some(e)))?;
    
    //         // let response_fields = types::incoming_response_headers(incoming_response);
    //         // let response_headers = Self::fields_to_header_map(response_fields);
    //         // types::drop_fields(headers);
    //         // types::drop_fields(response_fields);
    //         // let response_body_stream = types::incoming_response_consume(incoming_response)
    //         //     .map_err(|e| failure_point("incoming_response_consume", e))?;
    //         // let response_body: Body = response_body_stream.into();
    //         let req_opts = RequestOptions::new();
    //         //TODO(simon)
    //         // let _ = req_opts.set_connect_timeout_ms(self.inner.connect_timeout.map(|d| d.as_millis() as u64));
    //         // let _ = req_opts.set_first_byte_timeout_ms( timeout
    //         //     .or(self.inner.first_byte_timeout)
    //         //     .map(|d| d.as_millis() as u64));
    //         // let _ =  req_opts.set_between_bytes_timeout_ms(timeout
    //         //     .or(self.inner.between_bytes_timeout)
    //         //     .map(|d| d.as_millis() as u64));
    
    //         let future_response = outgoing_handler::handle(
    //             request,
    //             Some(req_opts),
    //         ).unwrap();
    
    //         let _ = types::OutgoingBody::finish(outgoing_body, None);
    
    //         let incoming_response = match future_response.get() {
    //             Some(result) => result.map_err(|_| failure_point("incoming response errored", ())).unwrap(),
    //             None => {
    //                 let pollable = future_response.subscribe();
    //                 let _ = poll::poll(&[&pollable]);
    //                 future_response
    //                     .get()
    //                     .expect("incoming response available")
    //                     .map_err(|e| failure_point("incoming response errored", e)).unwrap()
    //             }
    //         }
    //         // TODO: maybe anything that appears in the Result<_, E> position should impl
    //         // Error? anyway, just use its Debug here:
    //         .map_err(|_e| failure_point("{e:?}", ())).unwrap();
    
    //         drop(future_response);

    //         let mut builder = Response::builder();
    
    //         let status = incoming_response.status();
    //         builder.status(status);
    
    //         let headers_handle = incoming_response.headers();
    //         let _headers = headers_handle.entries();
    //         // let response_headers = Self::fields_to_header_map(&headers_handle); //TODO(simon)
    //         drop(headers_handle);
    
    //         let incoming_body = incoming_response
    //             .consume()
    //             .map_err(|()| failure_point("incoming response has no body stream", ())).unwrap();
    
    //         // drop(incoming_response);
    
    //         let input_stream = incoming_body.stream().unwrap();
    //         let input_stream_pollable = input_stream.subscribe();
    
    //         let mut body = Vec::new();
    //         loop {
    //             poll::poll(&[&input_stream_pollable]);
    
    //             let mut body_chunk = match input_stream.read(1024 * 1024) {
    //                 Ok(c) => c,
    //                 Err(streams::StreamError::Closed) => break,
    //                 Err(_e) => Err(failure_point("input_stream read failed", ())).unwrap(),
    //                 // Err(e) => Err(anyhow!("input_stream read failed: {e:?}"))?,
    //             };
    
    //             if !body_chunk.is_empty() {
    //                 body.append(&mut body_chunk);
    //             }
    //         }
    
    //         // let status_code = StatusCode::from_u16(status)
    //         //     .map_err(|e| crate::Error::new(crate::error::Kind::Decode, Some(e))).unwrap();
    
    //         // let response_body = Body::from(body);



    //         Ok(builder.body(BoxBody::from(body)))
    
    //         // Ok(Response::new(
    //         //     status_code,
    //         //     response_headers,
    //         //     response_body,
    //         //     incoming_response,
    //         //     url,
    //         // ))
    // }



// pub(crate) fn failure_point(s: &str, _: ()) -> crate::Error {
//     crate::Error::new(
//         Kind::Request,
//         Some(std::io::Error::new(ErrorKind::Other, s)),
//     )
// }



static WAKERS: Mutex<Vec<(poll::Pollable, Waker)>> = Mutex::new(Vec::new());

// https://github.com/fermyon/spin-rust-sdk/blob/main/src/http/executor.rs#L70
pub(crate) fn outgoing_body(body: OutgoingBody) -> impl Sink<Vec<u8>, Error = StreamError> {
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
    let pair = Rc::new(RefCell::new(Outgoing(Some((stream, body)))));

    sink::unfold((), {
        move |(), chunk: Vec<u8>| {
            future::poll_fn({
                let mut offset = 0;
                let mut flushing = false;
                let pair = pair.clone();

                move |context| {
                    let pair = pair.borrow();
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



// use spin_sdk::http::SendError;
// use spin_sdk::http::conversions::TryIntoOutgoingRequest;
// use spin_sdk::http::conversions::TryFromIncomingResponse;
// use super::executor;
// use futures_util::SinkExt;

// /// Send an outgoing request
// pub async fn send<I, O>(request: I) -> Result<O, SendError>
// where
//     I: TryIntoOutgoingRequest,
//     I::Error: Into<Box<dyn std::error::Error + Send + Sync>> + 'static,
//     O: TryFromIncomingResponse,
//     O::Error: Into<Box<dyn std::error::Error + Send + Sync>> + 'static,
// {
//     let (request, body_buffer) = I::try_into_outgoing_request(request)
//         .map_err(|e| SendError::RequestConversion(e.into()))?;
//     let response = if let Some(body_buffer) = body_buffer {
//         // It is part of the contract of the trait that implementors of `TryIntoOutgoingRequest`
//         // do not call `OutgoingRequest::write`` if they return a buffered body.
//         let mut body_sink = request.take_body();
//         let response = executor::outgoing_request_send(request);
//         body_sink.send(body_buffer).await.map_err(SendError::Io)?;
//         drop(body_sink);
//         response.await.map_err(SendError::Http)?
//     } else {
//         executor::outgoing_request_send(request)
//             .await
//             .map_err(SendError::Http)?
//     };

//     TryFromIncomingResponse::try_from_incoming_response(response)
//         .await
//         .map_err(|e: O::Error| SendError::ResponseConversion(e.into()))
// }