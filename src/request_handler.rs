use hyper::{Body, Method, Request, Response, StatusCode};
use hyper::body::HttpBody;
use serde::Deserialize;
use serde_json;
use std::sync::{Arc, Mutex};
use crate::Payload;

pub type Measurements = Arc<Mutex<Vec<Payload>>>;

#[derive(Deserialize)]
pub struct DbWritesEnableCommand {
    writes_enabled: bool,
    password: String,
}

#[derive(Clone)]
pub struct RequestHandler {
    db_writes_enabled: Arc<Mutex<bool>>,
    db_writes_password: String,
    graph_url: String,
    measurements: Measurements,
}

impl RequestHandler {
    pub fn new(
        db_writes_enabled: Arc<Mutex<bool>>, 
        db_writes_password: String, 
        graph_url: String, 
        measurements: Measurements
    ) -> Self {
        Self {
            db_writes_enabled,
            db_writes_password,
            graph_url,
            measurements,
        }
    }

    pub async fn route(&mut self, req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
        match (req.method(), req.uri().path()) {
            (&Method::GET, "/") => {
                // generate a simple http document based on db write status, the graph link and recent measurements
                let writes_enabled = self.db_writes_enabled.lock().unwrap();
                let measurements = self.measurements.lock().unwrap();
                let body = {
                    if let Some(last) = measurements.last() {
                        Body::from(format!(
                            r#"<!DOCTYPE html>
                            <html>
                            <p>Database writes enabled: {:} </p>
                            <p><a href="{:}">View graphs</a><p>
                            <p>Last measurement: {:} </p>
                            </html>"#
                            , writes_enabled, self.graph_url, last)
                        )
                    } else {
                        Body::from(format!(
                            r#"<!DOCTYPE html>
                            <html>
                            <p>Database writes enabled: {:} </p
                            <p><a href="{:}">View graphs</a><p>
                            </html>"#
                            , writes_enabled, self.graph_url)
                        )
                    }
                };

                let response = Response::builder()
                    .status(200)
                    .header("Content-Type", "text/html; charset=utf-8")
                    .body(body)
                    .unwrap();
                Ok(response)
            },
            (&Method::POST, "/") => {
                // limit request body size
                let upper = req.body().size_hint().upper().unwrap_or(u64::MAX);
                if upper > 1024 * 2 {
                    let mut resp = Response::new(Body::from("Body too large"));
                    *resp.status_mut() = hyper::StatusCode::PAYLOAD_TOO_LARGE;
                    return Ok(resp);
                }
                // await the whole request body
                let whole_body = hyper::body::to_bytes(req.into_body()).await?;
                // try to deserialize json payload
                match serde_json::from_slice::<DbWritesEnableCommand>(&whole_body) {
                    Ok(command) => {
                        if command.password == self.db_writes_password {
                            *self.db_writes_enabled.lock().unwrap() = command.writes_enabled;
                            Ok(Response::new(Body::from(format!("Writes enabled: {:}", command.writes_enabled))))
                        } else {
                            Ok(Response::new(Body::from("Wrong password")))
                        }
                    },
                    Err(_) => {
                        Ok(Response::new(Body::from("Invalid POST data")))
                    },
                }
            },
            _ => {
                let mut not_found = Response::default();
                *not_found.status_mut() = StatusCode::NOT_FOUND;
                Ok(not_found)
            }
        }
    }
}
