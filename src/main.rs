use std::collections::HashMap;

use actix_web::{error, web, App, Error, HttpResponse, HttpServer};
use anyhow::Context;
use cache::{memory_backend, CacheBackendFactory};
use clap::Parser;
use reqwest::Url;
use serde::Serialize;
use serde_json::{json, Value};
use tracing_subscriber::EnvFilter;

use crate::args::Args;
use crate::cache::redis_backend::RedisBackendFactory;
use crate::cache::CacheStatus;
use crate::json_rpc::{DefinedError, JsonRpcRequest, JsonRpcResponse, RequestId};
use crate::rpc_cache_handler::RpcCacheHandler;

mod args;
mod cache;
mod json_rpc;
mod rpc_cache_handler;
mod utils;

#[actix_web::post("/{chain}")]
async fn rpc_call(
    path: web::Path<(String,)>,
    data: web::Data<AppState>,
    body: web::Json<Value>,
) -> Result<HttpResponse, Error> {
    let (chain,) = path.into_inner();
    let chain_state = data
        .chains
        .get(&chain.to_uppercase())
        .ok_or_else(|| error::ErrorNotFound("endpoint not supported"))?;

    let (requests, is_single_request) = match body {
        web::Json(Value::Array(requests)) => (requests, false),
        web::Json(Value::Object(obj)) => (vec![Value::Object(obj)], true),
        _ => return JsonRpcResponse::from_error(None, DefinedError::InvalidRequest).into(),
    };

    let mut ordered_requests_result: Vec<Option<JsonRpcResponse>> = vec![None; requests.len()];
    let mut uncached_requests = vec![];
    let mut request_id_index_map: HashMap<RequestId, usize> = HashMap::new();

    // Scope the redis connection
    {
        let mut cache_backend = match chain_state.cache_factory.get_instance() {
            Ok(v) => v,
            Err(err) => {
                tracing::error!("fail to get cache backend because: {err:#}");
                return JsonRpcResponse::from_error(
                    None,
                    DefinedError::InternalError(Some(json!({
                        "error": "fail to get cache backend",
                        "reason": err.to_string(),
                    }))),
                )
                .into();
            }
        };

        for (index, request) in requests.into_iter().enumerate() {
            let (id, method, params) = match extract_single_request_info(request) {
                Ok(v) => v,
                Err((request_id, err)) => {
                    ordered_requests_result
                        .push(Some(JsonRpcResponse::from_error(request_id, err)));
                    continue;
                }
            };

            macro_rules! push_uncached_request_and_continue {
                () => {{
                    let rpc_request = RpcRequest::new_uncachable(index, id, method, params);
                    request_id_index_map.insert(rpc_request.id.clone(), uncached_requests.len());
                    uncached_requests.push(rpc_request);
                    continue;
                }};

                ($key: expr) => {{
                    let rpc_request = RpcRequest::new(index, id, method, params, $key);
                    request_id_index_map.insert(rpc_request.id.clone(), uncached_requests.len());
                    uncached_requests.push(rpc_request);
                    continue;
                }};
            }

            let cache_entry = match chain_state.cache_entries.get(&method) {
                Some(cache_entry) => cache_entry,
                None => {
                    tracing::warn!(method, "cache is not supported");
                    push_uncached_request_and_continue!()
                }
            };

            let params_key = match cache_entry.handler.extract_cache_key(&params) {
                Ok(Some(params_key)) => params_key,
                Ok(None) => push_uncached_request_and_continue!(),
                Err(err) => {
                    tracing::error!(
                        method,
                        params = format_args!("{}", params),
                        "fail to extract cache key: {err:#}",
                    );
                    push_uncached_request_and_continue!();
                }
            };

            match cache_backend.read(&method, &params_key) {
                Ok(CacheStatus::Cached { key, value }) => {
                    tracing::info!("cache hit for method {} with key {}", method, key);
                    ordered_requests_result[index] = Some(JsonRpcResponse::from_result(id, value));
                }
                Ok(CacheStatus::Missed { key }) => {
                    tracing::info!("cache missed for method {} with key {}", method, key);
                    push_uncached_request_and_continue!(key);
                }
                Err(err) => {
                    tracing::error!("fail to read cache because: {err:#}");
                    push_uncached_request_and_continue!();
                }
            }
        }
    }

    macro_rules! return_response {
        () => {
            return Ok(match is_single_request {
                true => ordered_requests_result[0].clone().unwrap().into(),
                false => HttpResponse::Ok().json(ordered_requests_result),
            })
        };
    }

    if uncached_requests.is_empty() {
        return_response!();
    }

    let rpc_result = utils::do_rpc_request(
        &data.http_client,
        chain_state.rpc_url.clone(),
        &uncached_requests,
    );

    let rpc_result = match rpc_result.await {
        Ok(v) => v,
        Err(err) => {
            tracing::error!("fail to make rpc request because: {}", err);

            for rpc_request in uncached_requests {
                ordered_requests_result[rpc_request.index] = Some(JsonRpcResponse::from_error(
                    Some(rpc_request.id),
                    DefinedError::InternalError(Some(json!({
                        "error": "fail to make rpc request to backend",
                        "reason": err.to_string(),
                    }))),
                ));
            }

            return_response!();
        }
    };

    let result_values = match rpc_result {
        Value::Array(v) => v,
        _ => {
            tracing::error!(
                "array is expected but we got invalid rpc response: {},",
                rpc_result.to_string()
            );

            for rpc_request in uncached_requests {
                ordered_requests_result[rpc_request.index] = Some(JsonRpcResponse::from_error(
                    Some(rpc_request.id),
                    DefinedError::InternalError(Some(json!({
                        "error": "invalid rpc response from backend",
                        "reason": "array is expected",
                        "response": rpc_result.to_string(),
                    }))),
                ));
            }

            return_response!();
        }
    };

    if result_values.len() != uncached_requests.len() {
        tracing::warn!(
            "rpc response length mismatch, expected: {}, got: {}",
            uncached_requests.len(),
            result_values.len()
        );
    }

    let mut cache_backend = match chain_state.cache_factory.get_instance() {
        Ok(v) => v,
        Err(err) => {
            tracing::error!("fail to get cache backend because: {}", err);

            for rpc_request in uncached_requests {
                ordered_requests_result[rpc_request.index] = Some(JsonRpcResponse::from_error(
                    Some(rpc_request.id),
                    DefinedError::InternalError(Some(json!({
                        "error": "fail to get cache backend",
                        "reason": err.to_string(),
                    }))),
                ));
            }

            return_response!();
        }
    };

    for (index, mut response) in result_values.into_iter().enumerate() {
        let rpc_request = match RequestId::try_from(response["id"].clone()) {
            Ok(id) if request_id_index_map.get(&id).is_some() => {
                &uncached_requests[*request_id_index_map.get(&id).unwrap()]
            }
            _ => {
                if index >= uncached_requests.len() {
                    tracing::warn!("rpc response has invalid id and fail to map to original request. response is ignored, response: {response}");
                    continue;
                }

                tracing::warn!(
                    "rpc response has invalid id. find a potential match from original request"
                );
                &uncached_requests[index]
            }
        };

        match response["error"].take() {
            Value::Null => {}
            error => {
                let response =
                    JsonRpcResponse::from_custom_error(Some(rpc_request.id.clone()), error);
                ordered_requests_result[rpc_request.index] = Some(response);
                continue;
            }
        }

        let result = response["result"].take();
        let response = JsonRpcResponse::from_result(rpc_request.id.clone(), result.clone());
        ordered_requests_result[rpc_request.index] = Some(response);

        let cache_key = match rpc_request.cache_key.clone() {
            Some(cache_key) => cache_key.clone(),
            None => continue,
        };

        // It's safe to unwrap here because if the cache system doesn't support this method, we have already
        // made the early return.
        let cache_entry = chain_state.cache_entries.get(&rpc_request.method).unwrap();

        let (can_cache, extracted_value) = match cache_entry.handler.extract_cache_value(&result) {
            Ok(v) => v,
            Err(err) => {
                tracing::error!("fail to extract cache value because: {}", err);

                ordered_requests_result[rpc_request.index] = Some(JsonRpcResponse::from_error(
                    Some(rpc_request.id.clone()),
                    DefinedError::InternalError(Some(json!({
                        "error": "fail to extract cache value",
                        "reason": err.to_string(),
                    }))),
                ));

                continue;
            }
        };

        if can_cache {
            let _ = cache_backend.write(&cache_key, &extracted_value.to_string());
        }
    }

    return_response!()
}

fn extract_single_request_info(
    mut raw_request: Value,
) -> Result<(RequestId, String, Value), (Option<RequestId>, DefinedError)> {
    let id = RequestId::try_from(raw_request["id"].take())
        .map_err(|_| (None, DefinedError::InvalidRequest))?;

    let method = match raw_request["method"].take() {
        Value::String(s) => s,
        _ => return Err((Some(id), DefinedError::MethodNotFound)),
    };

    let params = raw_request["params"].take();

    Ok((id, method, params))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();

    // Initialize tracing
    if std::env::var("RUST_LOG_FORMAT") == Ok("json".to_string()) {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .json()
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .init();
    }

    let mut app_state = AppState {
        chains: Default::default(),
        http_client: reqwest::Client::new(),
    };

    let handler_factories = rpc_cache_handler::factories();

    for (name, rpc_url) in args.endpoints.iter() {
        tracing::info!("Linked `{name}` to endpoint {rpc_url}");

        let chain_id = utils::get_chain_id(&reqwest::Client::new(), rpc_url.as_str())
            .await
            .expect("fail to get chain id");

        let cache_factory = new_cache_backend_factory(&args, chain_id)
            .expect("fail to create cache backend factory");

        let mut chain_state = ChainState {
            rpc_url: rpc_url.clone(),
            cache_entries: Default::default(),
            cache_factory,
        };

        for factory in &handler_factories {
            let handler = factory();
            chain_state
                .cache_entries
                .insert(handler.method_name().to_string(), CacheEntry { handler });
        }

        app_state.chains.insert(name.to_string(), chain_state);
    }

    let app_state = web::Data::new(app_state);

    tracing::info!("Server listening on {}:{}", args.bind, args.port);

    {
        let app_state = app_state.clone();

        HttpServer::new(move || App::new().service(rpc_call).app_data(app_state.clone()))
            .bind((args.bind, args.port))?
            .run()
            .await?;
    }

    tracing::info!("Server stopped");

    Ok(())
}

fn new_cache_backend_factory(
    args: &Args,
    chain_id: u64,
) -> anyhow::Result<Box<dyn CacheBackendFactory>> {
    let factory: Box<dyn CacheBackendFactory> = match &args.redis_url {
        Some(redis_url) => {
            tracing::info!("Using redis cache backend");

            let client =
                redis::Client::open(redis_url.as_ref()).context("fail to create redis client")?;

            let conn_pool = r2d2::Pool::builder()
                .max_size(300)
                .test_on_check_out(false)
                .build(client)
                .context("fail to create redis connection pool")?;
            let factory = RedisBackendFactory::new(chain_id, conn_pool);

            Box::new(factory)
        }
        None => {
            tracing::info!("Using in memory cache backend");
            Box::new(memory_backend::MemoryBackendFactory::new())
        }
    };

    Ok(factory)
}

struct ChainState {
    rpc_url: Url,
    cache_factory: Box<dyn CacheBackendFactory>,
    cache_entries: HashMap<String, CacheEntry>,
}

struct CacheEntry {
    handler: Box<dyn RpcCacheHandler>,
}

struct AppState {
    chains: HashMap<String, ChainState>,
    http_client: reqwest::Client,
}

#[derive(Debug, Clone)]
struct RpcRequest {
    index: usize,
    id: RequestId,
    method: String,
    params: Value,
    cache_key: Option<String>,
}

impl RpcRequest {
    fn new(index: usize, id: RequestId, method: String, params: Value, cache_key: String) -> Self {
        Self {
            index,
            id,
            method,
            params,
            cache_key: Some(cache_key),
        }
    }

    fn new_uncachable(index: usize, id: RequestId, method: String, params: Value) -> Self {
        Self {
            index,
            id,
            method,
            params,
            cache_key: None,
        }
    }
}

impl Serialize for RpcRequest {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        JsonRpcRequest::new(
            Some(self.id.clone()),
            self.method.clone(),
            self.params.clone(),
        )
        .serialize(serializer)
    }
}
