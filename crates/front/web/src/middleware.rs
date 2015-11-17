use std::io::{self, Read};
use iron::prelude::*;
use iron::status;
use iron::headers::Connection;
use iron::middleware::AfterMiddleware;
use super::config;

pub struct Catchall;

impl Catchall {
    pub fn new() -> Catchall {
        Catchall
    }
}

impl AfterMiddleware for Catchall {
    fn catch(&self, _: &mut Request, err: IronError) -> IronResult<Response> {
        match err.response.status {
            Some(status::NotFound) => Ok(err.response),
            _ => Err(err),
        }
    }
}

pub struct Drain;

impl Drain {
    pub fn new() -> Drain {
        Drain
    }

    fn drain(req: &mut Request, resp: &mut Response) {
        io::copy(&mut req.body.by_ref().take(config::REQUEST_SIZE), &mut io::sink()).unwrap();
        let mut buf = [0];
        if let Ok(n) = req.body.read(&mut buf) {
            if n > 0 {
                error!("Body too large, closing connection");
                resp.headers.set(Connection::close());
            }
        }
    }
}

impl AfterMiddleware for Drain {
    fn after(&self, req: &mut Request, mut resp: Response) -> IronResult<Response> {
        Drain::drain(req, &mut resp);
        Ok(resp)
    }

    fn catch(&self, req: &mut Request, mut err: IronError) -> IronResult<Response> {
        Drain::drain(req, &mut err.response);
        Err(err)
    }
}


