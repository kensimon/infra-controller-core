/*
 * SPDX-FileCopyrightText: Copyright (c) 2026 NVIDIA CORPORATION & AFFILIATES. All rights reserved.
 * SPDX-License-Identifier: Apache-2.0
 */
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use url::{Host, Url};

const MAX_URL_BYTES: usize = 2048;
const MAX_TRUST_DOMAIN_BYTES: usize = 253;
const MAX_ALLOWLIST_PATTERN_BYTES: usize = 512;

#[derive(Default, Debug, Clone)]
pub struct TrustDomainAllowList {
    entries: Vec<TrustDomainAllowlistPattern>,
}

impl TrustDomainAllowList {
    pub fn parse<S: AsRef<str>>(entries: &[S]) -> Result<Self, TrustDomainAllowListError> {
        Ok(Self {
            entries: entries
                .iter()
                .map(|s| s.as_ref().parse())
                .collect::<Result<Vec<_>, _>>()?,
        })
    }

    pub fn allows(&self, trust_domain: &str) -> bool {
        self.entries.is_empty() || self.entries.iter().any(|e| e.matches(trust_domain))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrustDomainAllowlistPattern {
    ExactDomain(String),
    SingleSubdomainWildcard { suffix: String },
    ManySubdomainWildcard { suffix: String },
}

impl TrustDomainAllowlistPattern {
    pub fn matches(&self, trust_domain: &str) -> bool {
        let trust_domain = trust_domain
            .trim()
            .trim_end_matches('.')
            .to_ascii_lowercase();
        if trust_domain.is_empty() {
            return false;
        }

        match self {
            Self::ExactDomain(domain) => trust_domain == *domain,
            Self::SingleSubdomainWildcard { suffix } => {
                let tail = format!(".{suffix}");
                trust_domain
                    .strip_suffix(&tail)
                    .is_some_and(|left| !left.is_empty() && !left.contains('.'))
            }
            Self::ManySubdomainWildcard { suffix } => {
                trust_domain == *suffix || trust_domain.ends_with(&format!(".{suffix}"))
            }
        }
    }
}

impl<'de> Deserialize<'de> for TrustDomainAllowList {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let strings = <Vec<String> as Deserialize>::deserialize(deserializer)?;
        TrustDomainAllowList::parse(&strings).map_err(serde::de::Error::custom)
    }
}

impl Serialize for TrustDomainAllowList {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.entries.serialize(serializer)
    }
}

impl Serialize for TrustDomainAllowlistPattern {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            TrustDomainAllowlistPattern::ExactDomain(domain) => s.serialize_str(domain.as_str()),
            TrustDomainAllowlistPattern::SingleSubdomainWildcard { suffix } => {
                s.serialize_str(&format!("*.{}", suffix).as_str())
            }
            TrustDomainAllowlistPattern::ManySubdomainWildcard { suffix } => {
                s.serialize_str(&format!("**.{}", suffix).as_str())
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TrustDomainAllowListError {
    #[error("Empty entry (after trim)")]
    EmptyEntry,
    #[error("pattern exceeds {max} bytes: {size}")]
    TooLarge { size: usize, max: usize },
    #[error("invalid pattern: {0}")]
    InvalidPattern(String),
    #[error("wildcards only as `*.` or `**.` prefix ({0})")]
    InvalidWildcards(String),
}

impl FromStr for TrustDomainAllowlistPattern {
    type Err = TrustDomainAllowListError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        let pattern = raw.trim().trim_end_matches('.').to_ascii_lowercase();
        if pattern.is_empty() {
            return Err(TrustDomainAllowListError::EmptyEntry);
        }
        if pattern.len() > MAX_ALLOWLIST_PATTERN_BYTES {
            return Err(TrustDomainAllowListError::TooLarge {
                size: pattern.len(),
                max: MAX_ALLOWLIST_PATTERN_BYTES,
            });
        }

        if let Some(suffix) = pattern.strip_prefix("**.") {
            if suffix.is_empty() || suffix.contains('*') {
                Err(TrustDomainAllowListError::InvalidPattern(raw.to_string()))
            } else {
                Ok(Self::ManySubdomainWildcard {
                    suffix: suffix.to_string(),
                })
            }
        } else if let Some(suffix) = pattern.strip_prefix("*.") {
            if suffix.is_empty() || suffix.contains('*') {
                Err(TrustDomainAllowListError::InvalidPattern(raw.to_string()))
            } else {
                Ok(Self::SingleSubdomainWildcard {
                    suffix: suffix.to_string(),
                })
            }
        } else if pattern.contains('*') {
            Err(TrustDomainAllowListError::InvalidWildcards(raw.to_string()))
        } else {
            Ok(Self::ExactDomain(pattern))
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedIssuer {
    pub identity: String,
    pub trust_domain: String,
}

#[derive(Debug, thiserror::Error)]
pub enum IssuerError {
    #[error("Invalid issuer URL: {0}")]
    Parse(#[from] IdentityParseError),
}

impl ValidatedIssuer {
    pub fn parse(raw: &str) -> Result<Self, IssuerError> {
        let identity_url =
            ValidatedIdentityUrl::parse(raw)?.require_scheme(&["http", "https", "spiffe"])?;
        Ok(ValidatedIssuer {
            identity: identity_url.canonicalized(),
            trust_domain: identity_url.trust_domain,
        })
    }

    pub fn into_string(self) -> String {
        self.identity
    }

    pub fn as_str(&self) -> &str {
        &self.identity
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedSubjectPrefix(String);

#[derive(Debug, thiserror::Error)]
pub enum SubjectPrefixError {
    #[error("Invalid subject prefix URL: {0}")]
    Parse(#[from] IdentityParseError),
    #[error(
        "Subject prefix trust domain {got:?} does not match issuer trust domain (expected {expected:?})"
    )]
    TrustDomainMismatch { expected: String, got: String },
}

impl ValidatedSubjectPrefix {
    pub fn parse(raw: &str, issuer: &ValidatedIssuer) -> Result<Self, SubjectPrefixError> {
        let raw = if raw.is_empty() {
            return Ok(Self(format!("spiffe://{}", issuer.trust_domain)));
        } else {
            raw
        };

        let spiffe_url = ValidatedIdentityUrl::parse(&raw)?.require_valid_spiffe_url()?;

        let canonicalized = spiffe_url.canonicalized();
        if raw.to_ascii_lowercase() != canonicalized.to_ascii_lowercase() {
            return Err(SubjectPrefixError::Parse(
                format!("SPIFFE URL is in non-canonical form: {raw}").into(),
            ));
        }

        if spiffe_url.trust_domain != issuer.trust_domain {
            return Err(SubjectPrefixError::TrustDomainMismatch {
                expected: issuer.trust_domain.clone(),
                got: spiffe_url.trust_domain,
            });
        }

        Ok(ValidatedSubjectPrefix(canonicalized))
    }

    pub fn into_string(self) -> String {
        self.0
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedTokenEndpoint {
    pub host: String,
}

#[derive(Debug, thiserror::Error)]
pub enum TokenEndpointError {
    #[error("Invalid token endpoint URL: {0}")]
    Parse(#[from] IdentityParseError),
}

impl ValidatedTokenEndpoint {
    pub fn parse(raw: &str) -> Result<Self, TokenEndpointError> {
        Ok(ValidatedTokenEndpoint {
            host: ValidatedIdentityUrl::parse(raw)?
                .require_scheme(&["http", "https"])?
                .trust_domain,
        })
    }
}

#[derive(Clone, Debug)]
struct ValidatedIdentityUrl {
    trust_domain: String,
    url: Url,
}

#[derive(thiserror::Error, Debug)]
pub enum IdentityParseError {
    #[error("{0}")]
    Formatted(String),
    #[error("{0}")]
    Static(&'static str),
}
impl From<&'static str> for IdentityParseError {
    fn from(value: &'static str) -> Self {
        Self::Static(value)
    }
}
impl From<String> for IdentityParseError {
    fn from(value: String) -> Self {
        Self::Formatted(value)
    }
}

impl ValidatedIdentityUrl {
    fn parse(raw: &str) -> Result<Self, IdentityParseError> {
        if raw.is_empty() {
            return Err("url is empty".into());
        }
        if raw.len() > MAX_URL_BYTES {
            return Err(format!("url exceeds maximum length ({MAX_URL_BYTES} bytes)").into());
        }
        if !raw.is_ascii() {
            return Err("must contain only ASCII characters".into());
        }
        if raw.bytes().any(|b| b < 0x20 || b == 0x7f) {
            return Err("must not contain control characters (disallowed)".into());
        }
        if raw.contains(['\\', '%', '#', ' ']) {
            return Err(
                "contains disallowed characters: must not contain spaces, '\\\\', '%', or '#'"
                    .into(),
            );
        }
        let mut url = Url::parse(&raw).map_err(|err| format!("invalid URL ({err})"))?;
        if url.query().is_some() {
            return Err("query is not allowed".into());
        }
        if url.fragment().is_some() {
            return Err("fragment is not allowed".into());
        }
        if !url.username().is_empty() || url.password().is_some() {
            return Err("URL must not contain userinfo".into());
        }
        let host = match url.host() {
            Some(Host::Domain(host)) if !host.is_empty() => host,
            Some(Host::Ipv4(_) | Host::Ipv6(_)) => {
                return Err(format!(
                    "trust domain must be a DNS hostname, not an IP address (got {:?})",
                    url
                )
                .into());
            }
            _ => return Err(format!("url must have a host: {}", url).into()),
        };
        if host.len() > MAX_TRUST_DOMAIN_BYTES {
            return Err(format!(
                "hostname part exceeds maximum length ({} bytes)",
                MAX_TRUST_DOMAIN_BYTES
            )
            .into());
        }
        let trust_domain = host.to_ascii_lowercase();
        url.set_host(Some(trust_domain.as_str()))
            .map_err(|err| format!("invalid URL ({})", err))?;
        Ok(Self { url, trust_domain })
    }

    fn require_valid_spiffe_url(self) -> Result<Self, IdentityParseError> {
        if self.url.scheme() != "spiffe" {
            return Err("must use the spiffe:// scheme".into());
        }
        if self.url.port().is_some() {
            return Err("must not include a port".into());
        }
        let path = self.url.path();
        if path.is_empty() || path == "/" {
            return Ok(self);
        }
        if !path.starts_with('/') {
            return Err("path must start with '/'".into());
        }
        if path.ends_with('/') {
            return Err("path must not end with '/' (use spiffe://<td> for root only)".into());
        }
        for segment in path.trim_start_matches('/').split('/') {
            if segment.is_empty() {
                return Err("path must not contain empty segments".into());
            }
            if segment == "." || segment == ".." {
                return Err("path must not use '.' or '..' segments".into());
            }
            if !segment
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
            {
                return Err(format!("path segment {segment:?} must match [a-zA-Z0-9._-]+").into());
            }
        }
        Ok(self)
    }

    fn require_scheme(self, allowed: &[&str]) -> Result<Self, IdentityParseError> {
        if allowed.contains(&self.url.scheme()) {
            Ok(self)
        } else {
            Err(format!(
                "Unexpected scheme {}: supported: {:?}",
                self.url.scheme(),
                allowed
            )
            .into())
        }
    }

    fn canonicalized(&self) -> String {
        let host = &self.trust_domain;
        let port = self
            .url
            .port()
            .map(|port| format!(":{port}"))
            .unwrap_or_default();
        let path = if self.url.path() == "/" {
            ""
        } else {
            self.url.path()
        };
        format!("{}://{host}{port}{path}", self.url.scheme())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolve_identity(
        issuer: &str,
        proto: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let issuer = ValidatedIssuer::parse(issuer)?;
        Ok(ValidatedSubjectPrefix::parse(proto.unwrap_or_default(), &issuer)?.into_string())
    }

    #[test]
    fn issuer_normalization_uses_url_parser() {
        assert_eq!(
            ValidatedIssuer::parse("HTTP://Issuer.EXAMPLE/path")
                .unwrap()
                .identity,
            "http://issuer.example/path"
        );
        assert_eq!(
            ValidatedIssuer::parse("SpIfFe://Issuer.EXAMPLE/bundle")
                .unwrap()
                .identity,
            "spiffe://issuer.example/bundle"
        );
        assert_eq!(
            ValidatedIssuer::parse("https://Issuer.EXAMPLE:8443/")
                .unwrap()
                .trust_domain,
            "issuer.example"
        );
    }

    #[test]
    fn issuer_preserves_path_case() {
        assert_eq!(
            ValidatedIssuer::parse("https://Issuer.EXAMPLE/OIDC/Callback")
                .unwrap()
                .identity,
            "https://issuer.example/OIDC/Callback"
        );
    }

    #[test]
    fn issuer_rejects_non_canonical_url_parts() {
        let err = ValidatedIssuer::parse("https://issuer.example/?q=1").unwrap_err();
        assert!(err.to_string().contains("query"), "{err}");

        let err = ValidatedIssuer::parse("https://user@issuer.example/").unwrap_err();
        assert!(err.to_string().contains("userinfo"), "{err}");

        let err = ValidatedIssuer::parse("https://127.0.0.1/").unwrap_err();
        assert!(err.to_string().contains("IP"), "{err}");
    }

    #[test]
    fn issuer_requires_expected_scheme() {
        let err = ValidatedIssuer::parse("issuer.example").unwrap_err();
        assert!(
            err.to_string().contains("relative URL without a base"),
            "{err}"
        );

        let err = ValidatedIssuer::parse("ftp://issuer.example/").unwrap_err();
        assert!(
            err.to_string().contains("http") || err.to_string().contains("spiffe"),
            "{err}"
        );
    }

    #[test]
    fn subject_prefix_defaults_to_spiffe_trust_domain() {
        assert_eq!(
            resolve_identity("https://my.idp.example", None).unwrap(),
            "spiffe://my.idp.example"
        );
        assert_eq!(
            resolve_identity("spiffe://my.idp.example/ns/x", None).unwrap(),
            "spiffe://my.idp.example"
        );
    }

    #[test]
    fn subject_prefix_is_canonicalized_from_url_components() {
        let prefix = resolve_identity(
            "https://issuer.example",
            Some("spiffe://ISSUER.EXAMPLE/workload"),
        )
        .unwrap();
        assert_eq!(prefix, "spiffe://issuer.example/workload");
    }

    #[test]
    fn subject_prefix_preserves_path_case() {
        let prefix = resolve_identity(
            "https://issuer.example",
            Some("spiffe://ISSUER.EXAMPLE/Workload/NodeA"),
        )
        .unwrap();
        assert_eq!(prefix, "spiffe://issuer.example/Workload/NodeA");
    }

    #[test]
    fn subject_prefix_rejects_bad_trust_domain_or_scheme() {
        let err =
            resolve_identity("https://issuer.example", Some("spiffe://other.example")).unwrap_err();
        assert!(err.to_string().contains("does not match"), "{err}");

        let err = resolve_identity(
            "https://issuer.example",
            Some("https://issuer.example/path"),
        )
        .unwrap_err();
        assert!(err.to_string().contains("spiffe://"), "{err}");
    }

    #[test]
    fn subject_prefix_rejects_non_canonical_forms() {
        for raw in [
            "spiffe://issuer.example/a%2Fb",
            "spiffe://issuer.example/a\\b",
            "spiffe://issuer.example/a b",
            "spiffe://issuer.example/a/",
            "spiffe://issuer.example/a//b",
            "spiffe://issuer.example/./a",
            "spiffe://issuer.example/../a",
            "spiffe://issuer.example:8443/a",
        ] {
            let err = resolve_identity("https://issuer.example", Some(raw)).unwrap_err();
            assert!(!err.to_string().is_empty(), "{raw}");
        }
    }

    #[test]
    fn subject_prefix_length_limit_is_enforced() {
        let base = "spiffe://issuer.example";
        let prefix = format!(
            "{base}{}",
            "x".repeat(MAX_URL_BYTES.saturating_sub(base.len()) + 1)
        );
        let err = resolve_identity("https://issuer.example", Some(&prefix)).unwrap_err();
        assert!(err.to_string().contains("maximum length"), "{err}");
    }

    #[test]
    fn allowlist_matching_and_validation_work() {
        let list = vec![
            "idp.example.com",
            "*.tenant.example.net",
            "**.corp.internal",
        ];
        let patterns = TrustDomainAllowList::parse(&list).unwrap();

        assert!(patterns.allows("idp.example.com"));
        assert!(patterns.allows("auth.tenant.example.net"));
        assert!(patterns.allows("a.b.corp.internal"));
        assert!(!patterns.allows("auth.app.tenant.example.net"));
        assert!(!patterns.allows("corp.internal.evil.com"));
    }

    #[test]
    fn allowlist_matches_apex_trailing_dot_and_mixed_case() {
        let patterns =
            TrustDomainAllowList::parse(&["  LOGIN.EXAMPLE.COM.  ", "**.Corp.Internal"]).unwrap();

        assert!(patterns.allows("login.example.com"));
        assert!(patterns.allows("LOGIN.EXAMPLE.COM."));
        assert!(patterns.allows("corp.internal"));
        assert!(patterns.allows("A.B.CORP.INTERNAL"));
    }

    #[test]
    fn allowlist_single_star_only_matches_one_label() {
        let patterns = TrustDomainAllowList::parse(&["*.something.net"]).unwrap();

        assert!(patterns.allows("auth.something.net"));
        assert!(!patterns.allows("something.net"));
        assert!(!patterns.allows("a.b.something.net"));
        assert!(!patterns.allows("notsomething.net"));
    }

    #[test]
    fn allowlist_double_star_requires_dot_boundary() {
        let patterns = TrustDomainAllowList::parse(&["**.internal.example"]).unwrap();

        assert!(patterns.allows("internal.example"));
        assert!(patterns.allows("x.internal.example"));
        assert!(!patterns.allows("api.internal.example.com"));
        assert!(!patterns.allows("not-relevant.internal.example.evil.com"));
    }

    #[test]
    fn allowlist_rejects_invalid_patterns() {
        assert!(TrustDomainAllowList::parse(&["*"]).is_err());
        assert!(TrustDomainAllowList::parse(&["**"]).is_err());
        assert!(TrustDomainAllowList::parse(&["*."]).is_err());
        assert!(TrustDomainAllowList::parse(&["**."]).is_err());
        assert!(TrustDomainAllowList::parse(&["foo*bar"]).is_err());
        assert!(TrustDomainAllowList::parse(&["**.foo.*.com"]).is_err());
        assert!(TrustDomainAllowList::parse(&["*.foo*bar.com"]).is_err());
        assert!(TrustDomainAllowList::parse(&["   "]).is_err());
        assert!(TrustDomainAllowList::parse(&["  \t "]).is_err());
    }

    #[test]
    fn token_endpoint_host_extraction_requires_http_or_https() {
        assert_eq!(
            ValidatedTokenEndpoint::parse("https://auth.example.com/oauth/token")
                .unwrap()
                .host,
            "auth.example.com"
        );
        assert_eq!(
            ValidatedTokenEndpoint::parse("http://auth.example:8080/token")
                .unwrap()
                .host,
            "auth.example"
        );

        let err = ValidatedTokenEndpoint::parse("spiffe://trust.example/path").unwrap_err();
        assert!(
            err.to_string().contains("http") && err.to_string().contains("https"),
            "{err}"
        );
    }
}
