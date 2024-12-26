use reqwest::header::{
    HeaderMap, HeaderValue, ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED, RETRY_AFTER,
};
use reqwest::{Response, StatusCode};
use std::future::Future;
use std::time::{Duration, Instant};
use time::format_description::well_known::Rfc2822;
use time::OffsetDateTime;

/// waiting 6 hours after a 429 should be slow enough for everyone
const DEFAULT_BACKOFF: Duration = Duration::from_secs(6 * 60 * 60);
const ONE_SEC: Duration = Duration::from_secs(1);

pub enum FetchPlanInput {
    Retry {
        time: Instant,
        headers: CacheHeaders,
    },
    WithCache {
        headers: CacheHeaders,
    },
}

impl FetchPlanInput {
    #[allow(dead_code)]
    pub fn into_cache_headers(self) -> CacheHeaders {
        match self {
            FetchPlanInput::Retry { headers, .. } => headers,
            FetchPlanInput::WithCache { headers } => headers,
        }
    }
}

#[derive(Default, Debug)]
pub struct CacheHeaders {
    etag: Option<String>,
    last_modified: Option<OffsetDateTime>,
}

impl CacheHeaders {
    pub fn from_headers(headers: &HeaderMap) -> CacheHeaders {
        let etag = headers
            .get(ETAG)
            .and_then(|header| header.to_str().ok())
            .map(String::from);
        let last_modified = headers
            .get(LAST_MODIFIED)
            .and_then(|header| header.to_str().ok())
            .and_then(|s| OffsetDateTime::parse(s, &Rfc2822).ok());
        CacheHeaders {
            etag,
            last_modified,
        }
    }

    pub fn set_headers(&self, headers: &mut HeaderMap) {
        match (&self.last_modified, &self.etag) {
            (_, Some(etag)) => {
                headers.insert(
                    IF_NONE_MATCH,
                    HeaderValue::from_str(etag).expect("malformed etag"),
                );
            }
            (Some(last_modified), None) => {
                headers.insert(
                    IF_MODIFIED_SINCE,
                    HeaderValue::from_str(&last_modified.format(&Rfc2822).unwrap()).unwrap(),
                );
            }
            _ => {}
        }
    }

    pub fn headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        self.set_headers(&mut headers);
        headers
    }
}

pub struct FetchPlan {
    pub time: Instant,
    pub headers: CacheHeaders,
}

impl FetchPlan {
    pub fn is_elapsed(&self) -> bool {
        Instant::now() >= self.time
    }
}

impl Default for FetchPlan {
    fn default() -> FetchPlan {
        FetchPlan {
            time: Instant::now(),
            headers: CacheHeaders::default(),
        }
    }
}

/// plan the next fetch, either on startup or right after we finished the previous fetch
pub fn next_fetch(base_interval: Duration, last_result: Option<FetchPlanInput>) -> FetchPlan {
    let now = Instant::now();
    match last_result {
        Some(FetchPlanInput::Retry { time, headers }) => FetchPlan {
            time: now.max(time),
            headers,
        },
        Some(FetchPlanInput::WithCache { headers }) => FetchPlan {
            time: now + base_interval,
            headers,
        },
        None => FetchPlan {
            time: now + base_interval,
            headers: CacheHeaders::default(),
        },
    }
}

pub enum FetchResponse<T, E> {
    Retry {
        time: Instant,
        headers: CacheHeaders,
    },
    Ok {
        headers: CacheHeaders,
        response: T,
    },
    Error {
        headers: CacheHeaders,
        error: E,
    },
}

impl<T, E> FetchResponse<T, E> {
    #[allow(dead_code)]
    pub fn plan(self) -> FetchPlanInput {
        match self {
            FetchResponse::Retry { time, headers } => FetchPlanInput::Retry { time, headers },
            FetchResponse::Ok { headers, .. } => FetchPlanInput::WithCache { headers },
            FetchResponse::Error { headers, .. } => FetchPlanInput::WithCache { headers },
        }
    }

    pub fn into_result(self) -> Result<(Option<T>, FetchPlanInput), (E, FetchPlanInput)> {
        match self {
            FetchResponse::Retry { time, headers } => {
                Ok((None, FetchPlanInput::Retry { time, headers }))
            }
            FetchResponse::Ok { headers, response } => {
                Ok((Some(response), FetchPlanInput::WithCache { headers }))
            }
            FetchResponse::Error { headers, error } => {
                Err((error, FetchPlanInput::WithCache { headers }))
            }
        }
    }
}

impl<E> FetchResponse<Response, E> {
    pub fn from_result(result: Result<Response, E>) -> FetchResponse<Response, E> {
        match result {
            Ok(response) => {
                let cache_header = CacheHeaders::from_headers(response.headers());
                if response.status() == StatusCode::TOO_MANY_REQUESTS {
                    let after = response
                        .headers()
                        .get(RETRY_AFTER)
                        .and_then(|header| header.to_str().ok())
                        .and_then(|str| str.parse::<u64>().ok())
                        .map(Duration::from_secs)
                        .unwrap_or(DEFAULT_BACKOFF);
                    FetchResponse::Retry {
                        time: Instant::now() + after + ONE_SEC,
                        headers: cache_header,
                    }
                } else {
                    FetchResponse::Ok {
                        headers: cache_header,
                        response,
                    }
                }
            }
            Err(err) => FetchResponse::Error {
                error: err,
                headers: CacheHeaders::default(),
            },
        }
    }

    pub fn check_status_code<Fc, Fs>(self, client_error: Fc, server_error: Fs) -> Self
    where
        Fc: Fn(StatusCode) -> E,
        Fs: Fn(StatusCode) -> E,
    {
        match self {
            FetchResponse::Ok { headers, response } => {
                let status = response.status();
                if status.is_client_error() {
                    FetchResponse::Error {
                        error: client_error(status),
                        headers,
                    }
                } else if status.is_server_error() {
                    FetchResponse::Error {
                        error: server_error(status),
                        headers,
                    }
                } else {
                    FetchResponse::Ok { headers, response }
                }
            }
            rest => rest,
        }
    }
}

impl<T, E> FetchResponse<T, E> {
    pub async fn map<U, Fut, F>(self, f: F) -> FetchResponse<U, E>
    where
        Fut: Future<Output = U>,
        F: Fn(T) -> Fut,
    {
        match self {
            FetchResponse::Retry { time, headers } => FetchResponse::Retry { time, headers },
            FetchResponse::Ok { headers, response } => FetchResponse::Ok {
                headers,
                response: f(response).await,
            },
            FetchResponse::Error { error, headers } => FetchResponse::Error { error, headers },
        }
    }
    pub fn map_err<U, F>(self, f: F) -> FetchResponse<T, U>
    where
        F: Fn(E) -> U,
    {
        match self {
            FetchResponse::Retry { time, headers } => FetchResponse::Retry { time, headers },
            FetchResponse::Ok { headers, response } => FetchResponse::Ok { headers, response },
            FetchResponse::Error { error, headers } => FetchResponse::Error {
                error: f(error),
                headers,
            },
        }
    }
}

impl<T, E> FetchResponse<Result<T, E>, E> {
    pub fn flatten(self) -> FetchResponse<T, E> {
        match self {
            FetchResponse::Retry { time, headers } => FetchResponse::Retry { time, headers },
            FetchResponse::Ok {
                headers,
                response: Ok(response),
            } => FetchResponse::Ok { headers, response },
            FetchResponse::Ok {
                headers,
                response: Err(error),
            } => FetchResponse::Error { error, headers },
            FetchResponse::Error { error, headers } => FetchResponse::Error { error, headers },
        }
    }
}
