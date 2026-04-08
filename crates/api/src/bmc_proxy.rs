/*
 * SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
 * SPDX-License-Identifier: Apache-2.0
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std::collections::HashSet;
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::State;
use axum::http::{HeaderMap, Method, Request, Response, StatusCode, Uri};
use axum::middleware::{Next, from_fn_with_state};
use axum::response::IntoResponse;
use axum::routing::{any, get};
use bytes::Bytes;
use forge_secrets::credentials::CredentialManager;
use opentelemetry::metrics::Meter;
use sqlx::PgPool;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use crate::auth::{AuthContext, Principal};
use crate::cfg::file::{AuthConfig, BmcProxyConfig};
use crate::dynamic_settings::DynamicSettings;
use crate::listener::{self, ApiListenMode};

#[derive(Clone)]
struct BmcProxyState {
    database_connection: PgPool,
    credential_manager: Arc<dyn CredentialManager>,
    dynamic_settings: DynamicSettings,
    allowed_principals: Arc<HashSet<String>>,
}

pub async fn start(
    join_set: &mut JoinSet<()>,
    db_pool: PgPool,
    credential_manager: Arc<dyn CredentialManager>,
    dynamic_settings: DynamicSettings,
    listen_mode: ApiListenMode,
    auth_config: &Option<AuthConfig>,
    proxy_config: &BmcProxyConfig,
    meter: Meter,
    cancel_token: CancellationToken,
) -> eyre::Result<()> {
    let state = BmcProxyState {
        database_connection: db_pool,
        credential_manager,
        dynamic_settings,
        allowed_principals: Arc::new(proxy_config.allowed_principals.iter().cloned().collect()),
    };

    let app = Router::new()
        .route("/", get(root_url))
        .route("/{*path}", any(proxy_request))
        .with_state(state.clone())
        .layer(from_fn_with_state(
            state.clone(),
            authorize_proxy_request,
        ))
        .layer(listener::cert_description_layer(auth_config)?);

    listener::serve_router(
        join_set,
        app,
        listen_mode,
        proxy_config.listen,
        meter,
        cancel_token,
    )
    .await
}

async fn root_url() -> &'static str {
    const ROOT_CONTENTS: &str = if carbide_version::literal!(build_version).is_empty() {
        "Carbide BMC proxy development build\n"
    } else {
        concat!("Carbide BMC proxy ", carbide_version::literal!(build_version), "\n")
    };
    ROOT_CONTENTS
}

async fn proxy_request(
    State(state): State<BmcProxyState>,
    request: Request<Body>,
) -> Result<Response<Body>, Response<Body>> {
    let (parts, body) = request.into_parts();
    let target_ip = forwarded_host_ip(&parts.headers)
        .map_err(|e| error_response((StatusCode::BAD_REQUEST, e.to_string()).into()))?;

    let path_and_query = parts
        .uri
        .path_and_query()
        .cloned()
        .ok_or_else(|| error_response((StatusCode::BAD_REQUEST, "missing path").into()))?;
    let target_uri = Uri::builder()
        .scheme("https")
        .authority(target_ip.to_string())
        .path_and_query(path_and_query)
        .build()
        .map_err(|e| {
            error_response(
                (
                    StatusCode::BAD_REQUEST,
                    format!("invalid target uri for proxied request: {e}"),
                )
                    .into(),
            )
        })?;

    let (metadata, upstream_uri, mut upstream_headers, http_client) =
        crate::handlers::redfish::create_client(
            target_uri,
            &state.database_connection,
            state.credential_manager.as_ref(),
            &state.dynamic_settings.bmc_proxy,
        )
        .await
        .map_err(|e| error_response((StatusCode::BAD_GATEWAY, e.to_string()).into()))?;

    copy_request_headers(&parts.headers, &mut upstream_headers);

    let body = to_bytes(body, usize::MAX)
        .await
        .map_err(|e| error_response((StatusCode::BAD_REQUEST, e.to_string()).into()))?;

    let mut upstream_request = http_client
        .request(parts.method.clone(), upstream_uri.to_string())
        .basic_auth(metadata.user, Some(metadata.password))
        .headers(upstream_headers);

    if method_supports_body(&parts.method) {
        upstream_request = upstream_request.body(body);
    }

    let upstream_response = upstream_request
        .send()
        .await
        .map_err(|e| error_response((StatusCode::BAD_GATEWAY, e.to_string()).into()))?;

    let status = upstream_response.status();
    let headers = upstream_response.headers().clone();
    let body = upstream_response
        .bytes()
        .await
        .map_err(|e| error_response((StatusCode::BAD_GATEWAY, e.to_string()).into()))?;

    Ok(build_response(status, &headers, body))
}

async fn authorize_proxy_request(
    State(state): State<BmcProxyState>,
    request: Request<Body>,
    next: Next,
) -> Result<Response<Body>, StatusCode> {
    let auth_context = request.extensions().get::<AuthContext>().ok_or_else(|| {
        tracing::warn!(
            "authorize_proxy_request found a request with no AuthContext in its extensions"
        );
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut present_principals = auth_context
        .principals
        .iter()
        .map(Principal::as_identifier)
        .collect::<Vec<_>>();
    present_principals.push(Principal::Anonymous.as_identifier());

    let allowed = present_principals
        .iter()
        .any(|principal| state.allowed_principals.contains(principal));

    if allowed {
        Ok(next.run(request).await)
    } else {
        tracing::info!(
            allowed_principals = ?state.allowed_principals,
            present_principals = ?present_principals,
            path = request.uri().path(),
            "Request denied by BMC proxy principal allow-list"
        );
        Err(StatusCode::FORBIDDEN)
    }
}

fn build_response(status: reqwest::StatusCode, headers: &reqwest::header::HeaderMap, body: Bytes) -> Response<Body> {
    let mut response = Response::builder().status(status);
    for (name, value) in headers {
        if is_hop_by_hop_header(name.as_str()) || name == reqwest::header::CONTENT_LENGTH {
            continue;
        }
        response = response.header(name, value);
    }
    response.body(Body::from(body)).unwrap()
}

fn copy_request_headers(source: &HeaderMap, dest: &mut HeaderMap) {
    for (name, value) in source {
        if is_hop_by_hop_header(name.as_str())
            || *name == axum::http::header::HOST
            || *name == axum::http::header::AUTHORIZATION
            || name.as_str().eq_ignore_ascii_case("forwarded")
            || *name == axum::http::header::CONTENT_LENGTH
        {
            continue;
        }
        dest.append(name.clone(), value.clone());
    }
}

fn method_supports_body(method: &Method) -> bool {
    !matches!(*method, Method::GET | Method::HEAD)
}

fn is_hop_by_hop_header(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailer"
            | "transfer-encoding"
            | "upgrade"
    )
}

fn forwarded_host_ip(headers: &HeaderMap) -> Result<IpAddr, eyre::Report> {
    let values = headers.get_all("forwarded");
    for raw_value in values {
        let raw_value = raw_value.to_str()?;
        for element in raw_value.split(',') {
            for pair in element.split(';') {
                let Some((key, value)) = pair.trim().split_once('=') else {
                    continue;
                };
                if !key.trim().eq_ignore_ascii_case("host") {
                    continue;
                }
                return parse_forwarded_host_value(value.trim());
            }
        }
    }

    Err(eyre::eyre!("missing Forwarded host parameter"))
}

fn parse_forwarded_host_value(value: &str) -> Result<IpAddr, eyre::Report> {
    let value = value.trim_matches('"');

    if let Ok(ip) = IpAddr::from_str(value) {
        return Ok(ip);
    }

    if let Some(rest) = value.strip_prefix('[')
        && let Some((host, _)) = rest.split_once(']')
    {
        return IpAddr::from_str(host).map_err(Into::into);
    }

    if let Some((host, _port)) = value.rsplit_once(':')
        && let Ok(ip) = IpAddr::from_str(host)
    {
        return Ok(ip);
    }

    Err(eyre::eyre!("Forwarded host must be an IP address"))
}

fn error_response(error: ProxyError) -> Response<Body> {
    (error.status, error.message).into_response()
}

struct ProxyError {
    status: StatusCode,
    message: String,
}

impl From<(StatusCode, String)> for ProxyError {
    fn from((status, message): (StatusCode, String)) -> Self {
        Self { status, message }
    }
}

impl From<(StatusCode, &'static str)> for ProxyError {
    fn from((status, message): (StatusCode, &'static str)) -> Self {
        Self {
            status,
            message: message.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{forwarded_host_ip, parse_forwarded_host_value};
    use axum::http::{HeaderMap, HeaderName, HeaderValue};
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    use std::str::FromStr;

    #[test]
    fn parses_forwarded_ipv4() {
        assert_eq!(
            parse_forwarded_host_value("10.0.0.5").unwrap(),
            IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5))
        );
    }

    #[test]
    fn parses_forwarded_ipv6_with_port() {
        assert_eq!(
            parse_forwarded_host_value("\"[2001:db8::1]:443\"").unwrap(),
            IpAddr::V6(Ipv6Addr::from_str("2001:db8::1").unwrap())
        );
    }

    #[test]
    fn finds_forwarded_host_among_parameters() {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("forwarded"),
            HeaderValue::from_static("proto=https;host=10.1.2.3;for=10.0.0.1"),
        );
        assert_eq!(
            forwarded_host_ip(&headers).unwrap(),
            IpAddr::V4(Ipv4Addr::new(10, 1, 2, 3))
        );
    }
}
