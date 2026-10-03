//! The citations rung (9122): every DOI, arXiv id and http(s) URL an
//! artefact cites must resolve.
//!
//! Model-written citations are often fabricated, and nothing else checks
//! them. [`check_citations`] extracts each identifier from a rung's
//! artefacts ([`extract_citations`]) and looks it up through a
//! [`CitationResolver`]. One that does not resolve fails the rung, and the
//! verdict names it so the next attempt can fix it. One that cannot be
//! looked up (no network, a timeout, a server error) leaves the rung
//! skipped, never passed. The verdict's detail lists every lookup, so that
//! audits can check the checker.
//!
//! [`HttpCitationResolver`] looks identifiers up as
//! `tmp/cybernetic-harness/tools/refcheck.py` does: a DOI at Crossref, then at
//! DataCite; an arXiv id at DataCite (`10.48550/arXiv.<id>`), since the arXiv
//! API refuses some query forms; a URL by `HEAD`, then `GET`. It goes through
//! roko's shared HTTP client, with a time limit per request, and keeps each
//! answer for the plan run. A cited URL is looked up only when roko's network
//! policy lets a network tool reach it (http or https, no private network),
//! since the artefact's author chose it. Titles are not compared yet: v1
//! checks that each identifier exists.

use std::collections::{BTreeSet, HashMap};
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use futures::stream::{self, StreamExt};
use regex::Regex;
use roko_agent::safety::network::{NetworkPolicy, check_url_with_policy};
use roko_core::Verdict;

/// Lookups in flight at once.
const CONCURRENT_LOOKUPS: usize = 4;

/// The time limit of one lookup request.
const LOOKUP_TIMEOUT: Duration = Duration::from_secs(15);

/// One identifier an artefact cites.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Citation {
    /// A DOI, such as `10.1145/3442188.3445922`, in lower case.
    Doi(String),
    /// An arXiv id without its version: new style (`2303.08774`) or old
    /// style (`cs/0112017`).
    Arxiv(String),
    /// An http(s) URL.
    Url(String),
}

impl std::fmt::Display for Citation {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Doi(doi) => write!(formatter, "doi:{doi}"),
            Self::Arxiv(id) => write!(formatter, "arXiv:{id}"),
            Self::Url(url) => formatter.write_str(url),
        }
    }
}

static DOI: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b10\.\d{4,9}/[-._;()/:A-Za-z0-9]+").expect("valid DOI regex"));
static ARXIV: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r"(?i:arxiv)(?:\.org/(?:abs|pdf)/|:\s*|\s+)",
        r"(\d{4}\.\d{4,5}|[a-z][a-z\-]*(?:\.[A-Z]{2})?/\d{7})(?:v\d+)?",
    ))
    .expect("valid arXiv regex")
});
static URL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"https?://[^\s<>"'`\])}]+"#).expect("valid URL regex"));

/// Every citation in `text`, each once. A doi.org or arxiv.org link counts
/// as the DOI or the arXiv id it names, not as a URL.
#[must_use]
pub fn extract_citations(text: &str) -> BTreeSet<Citation> {
    let mut citations = BTreeSet::new();
    for found in DOI.find_iter(text) {
        let doi = trim_reference(found.as_str()).to_ascii_lowercase();
        citations.insert(Citation::Doi(doi));
    }
    for captures in ARXIV.captures_iter(text) {
        citations.insert(Citation::Arxiv(captures[1].to_string()));
    }
    for found in URL.find_iter(text) {
        let url = trim_reference(found.as_str());
        if !names_an_identifier(url) {
            citations.insert(Citation::Url(url.to_string()));
        }
    }
    citations
}

/// `found` without the punctuation a sentence or a bracket puts after it.
fn trim_reference(found: &str) -> &str {
    const TRAILING: [char; 8] = ['.', ',', ';', ':', '!', '?', '\'', '"'];
    let mut trimmed = found.trim_end_matches(TRAILING);
    // A closing parenthesis the reference does not open belongs to the text.
    while trimmed.ends_with(')') && trimmed.matches('(').count() < trimmed.matches(')').count() {
        trimmed = trimmed[..trimmed.len() - 1].trim_end_matches(TRAILING);
    }
    trimmed
}

/// Whether `url` is a doi.org or arxiv.org link, which names a DOI or an
/// arXiv id that [`extract_citations`] finds on its own.
fn names_an_identifier(url: &str) -> bool {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let host = host.trim_start_matches("www.").to_ascii_lowercase();
    matches!(
        host.as_str(),
        "doi.org" | "dx.doi.org" | "arxiv.org" | "export.arxiv.org"
    )
}

/// What one lookup found ([`CitationResolver::resolve`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    /// The identifier exists.
    Resolved {
        /// What answered for it, such as `crossref` or `http 200`.
        source: String,
    },
    /// The identifier does not exist where it should.
    Unresolved {
        /// What the lookups answered, such as `crossref 404, datacite 404`.
        detail: String,
    },
}

/// Looks citations up.
#[async_trait]
pub trait CitationResolver: Send + Sync {
    /// Look `citation` up. `Err` means the lookup could not be made (no
    /// network, a timeout, a server error), which says nothing about the
    /// citation.
    async fn resolve(&self, citation: &Citation) -> Result<Resolution, String>;
}

/// Looks citations up over HTTP through roko's shared client.
///
/// It keeps each answer for the life of the resolver, one plan run. A lookup that
/// could not be made is not kept, so a later attempt tries it again.
#[derive(Debug, Default)]
pub struct HttpCitationResolver {
    answers: Mutex<HashMap<Citation, Resolution>>,
}

impl HttpCitationResolver {
    /// A resolver that has kept no answer yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl CitationResolver for HttpCitationResolver {
    async fn resolve(&self, citation: &Citation) -> Result<Resolution, String> {
        let kept = self
            .answers
            .lock()
            .ok()
            .and_then(|answers| answers.get(citation).cloned());
        if let Some(answer) = kept {
            return Ok(answer);
        }
        let answer = match citation {
            Citation::Doi(doi) => resolve_doi(doi).await?,
            Citation::Arxiv(id) => resolve_arxiv(id).await?,
            Citation::Url(url) => {
                check_url_with_policy(url, &cited_url_policy())
                    .map_err(|error| format!("roko's network policy refuses it: {error}"))?;
                resolve_url(url).await?
            }
        };
        if let Ok(mut answers) = self.answers.lock() {
            answers.insert(citation.clone(), answer.clone());
        }
        Ok(answer)
    }
}

/// The network policy a cited URL must pass before it is looked up: roko's
/// default for network tools (no private network, no denied host), with
/// plain http allowed, as many citations use it.
fn cited_url_policy() -> NetworkPolicy {
    NetworkPolicy {
        allow_schemes: vec!["https".to_string(), "http".to_string()],
        ..NetworkPolicy::default()
    }
}

/// The status `url` answers a `GET` with, or with `get` false a `HEAD`.
async fn http_status(url: &str, get: bool) -> Result<u16, String> {
    let client = roko_agent::provider::shared_http_client();
    let request = if get {
        client.get(url)
    } else {
        client.head(url)
    };
    let response = request
        .timeout(LOOKUP_TIMEOUT)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    Ok(response.status().as_u16())
}

/// A status, or why there is none, for a lookup's detail.
fn status_text(status: &Result<u16, String>) -> String {
    match status {
        Ok(status) => status.to_string(),
        Err(error) => error.clone(),
    }
}

fn resolved(source: impl Into<String>) -> Resolution {
    Resolution::Resolved {
        source: source.into(),
    }
}

/// A DOI resolves at Crossref, or at DataCite, which registers DOIs that
/// Crossref does not (arXiv's among them). It does not resolve when both
/// answer 404.
async fn resolve_doi(doi: &str) -> Result<Resolution, String> {
    let crossref = http_status(&format!("https://api.crossref.org/works/{doi}"), true).await;
    if crossref == Ok(200) {
        return Ok(resolved("crossref"));
    }
    let datacite = http_status(&format!("https://api.datacite.org/dois/{doi}"), true).await;
    match (&crossref, &datacite) {
        (_, Ok(200)) => Ok(resolved("datacite")),
        (Ok(404), Ok(404)) => Ok(Resolution::Unresolved {
            detail: "crossref 404, datacite 404".to_string(),
        }),
        _ => Err(format!(
            "crossref {}, datacite {}",
            status_text(&crossref),
            status_text(&datacite)
        )),
    }
}

/// An arXiv id resolves at DataCite, as the DOI arXiv registers for it.
async fn resolve_arxiv(id: &str) -> Result<Resolution, String> {
    let url = format!("https://api.datacite.org/dois/10.48550/arXiv.{id}");
    match http_status(&url, true).await {
        Ok(200) => Ok(resolved("datacite")),
        Ok(404) => Ok(Resolution::Unresolved {
            detail: "datacite 404".to_string(),
        }),
        status => Err(format!("datacite {}", status_text(&status))),
    }
}

/// A URL resolves when a `HEAD`, or failing that a `GET`, answers below 400,
/// and does not when the `GET` answers 404 or 410. Any other refusal (403,
/// 429, a server error) says nothing either way.
async fn resolve_url(url: &str) -> Result<Resolution, String> {
    if let Ok(status) = http_status(url, false).await
        && status < 400
    {
        return Ok(resolved(format!("http {status}")));
    }
    match http_status(url, true).await? {
        status if status < 400 => Ok(resolved(format!("http {status}"))),
        status @ (404 | 410) => Ok(Resolution::Unresolved {
            detail: format!("http {status}"),
        }),
        status => Err(format!("http {status}")),
    }
}

/// The citations rung `gate`'s verdict on `artefacts`, each a path and its text.
///
/// It fails, naming them, when citations do not resolve; else it is
/// skipped when a citation could not be looked up; else it passes. Its
/// detail lists every lookup.
pub async fn check_citations(
    gate: &str,
    artefacts: &[(String, String)],
    resolver: &dyn CitationResolver,
) -> Verdict {
    let citations: BTreeSet<Citation> = artefacts
        .iter()
        .flat_map(|(_, text)| extract_citations(text))
        .collect();
    let lookups: Vec<(Citation, Result<Resolution, String>)> = stream::iter(citations)
        .map(|citation| async move {
            let answer = resolver.resolve(&citation).await;
            (citation, answer)
        })
        .buffered(CONCURRENT_LOOKUPS)
        .collect()
        .await;

    let detail = lookups
        .iter()
        .map(|(citation, answer)| match answer {
            Ok(Resolution::Resolved { source }) => format!("{citation}: resolves ({source})"),
            Ok(Resolution::Unresolved { detail }) => {
                format!("{citation}: does not resolve ({detail})")
            }
            Err(error) => format!("{citation}: not checked ({error})"),
        })
        .collect::<Vec<_>>()
        .join("\n");
    let unresolved: Vec<String> = lookups
        .iter()
        .filter(|(_, answer)| matches!(answer, Ok(Resolution::Unresolved { .. })))
        .map(|(citation, _)| citation.to_string())
        .collect();
    let unchecked = lookups.iter().filter(|(_, answer)| answer.is_err()).count();
    let total = lookups.len();
    let verdict = if !unresolved.is_empty() {
        Verdict::fail(
            gate,
            format!(
                "{} of {total} citations do not resolve: {}",
                unresolved.len(),
                unresolved.join(", ")
            ),
        )
    } else if unchecked > 0 {
        Verdict::skip(
            gate,
            format!("{unchecked} of {total} citations could not be looked up"),
        )
    } else {
        let mut verdict = Verdict::pass(gate);
        verdict.reason = format!("{total} citations resolve");
        verdict
    };
    verdict.with_detail(detail)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Knows `known` by their labels, and with `reachable` false cannot be
    /// reached at all.
    struct FakeResolver {
        known: Vec<&'static str>,
        reachable: bool,
    }

    #[async_trait]
    impl CitationResolver for FakeResolver {
        async fn resolve(&self, citation: &Citation) -> Result<Resolution, String> {
            if !self.reachable {
                return Err("network unreachable".to_string());
            }
            let label = citation.to_string();
            Ok(if self.known.contains(&label.as_str()) {
                resolved("fake")
            } else {
                Resolution::Unresolved {
                    detail: "fake 404".to_string(),
                }
            })
        }
    }

    const REAL: [&str; 3] = [
        "doi:10.48550/arxiv.1706.03762",
        "arXiv:2303.08774",
        "https://example.org/data",
    ];

    fn report() -> Vec<(String, String)> {
        vec![(
            "report.md".to_string(),
            "Attention (doi:10.48550/arXiv.1706.03762) and GPT-4 (arXiv:2303.08774v3). \
             Data: https://example.org/data. Survey: https://doi.org/10.9999/Fabricated.2024."
                .to_string(),
        )]
    }

    /// 9122: the rung extracts each DOI, arXiv id and URL once, a doi.org
    /// link as its DOI. One DOI no resolver knows fails the rung, which names
    /// it; when every citation resolves the rung passes; a resolver that
    /// cannot be reached leaves it skipped, not passed.
    #[tokio::test]
    async fn citation_rung_fails_unresolvable_doi() {
        let labels: Vec<String> = report()
            .iter()
            .flat_map(|(_, text)| extract_citations(text))
            .map(|citation| citation.to_string())
            .collect();
        assert_eq!(
            labels,
            [
                "doi:10.48550/arxiv.1706.03762",
                "doi:10.9999/fabricated.2024",
                "arXiv:2303.08774",
                "https://example.org/data"
            ]
        );

        let partial = FakeResolver {
            known: REAL.to_vec(),
            reachable: true,
        };
        let verdict = check_citations("rung[sources]", &report(), &partial).await;
        assert!(!verdict.passed && !verdict.skipped, "{verdict:?}");
        assert_eq!(
            verdict.reason,
            "1 of 4 citations do not resolve: doi:10.9999/fabricated.2024"
        );
        let detail = verdict.detail.unwrap_or_default();
        assert!(
            detail.contains("arXiv:2303.08774: resolves (fake)"),
            "{detail}"
        );

        let mut known = REAL.to_vec();
        known.push("doi:10.9999/fabricated.2024");
        let complete = FakeResolver {
            known,
            reachable: true,
        };
        let verdict = check_citations("rung[sources]", &report(), &complete).await;
        assert!(verdict.passed, "{verdict:?}");
        assert_eq!(verdict.reason, "4 citations resolve");

        let offline = FakeResolver {
            known: REAL.to_vec(),
            reachable: false,
        };
        let verdict = check_citations("rung[sources]", &report(), &offline).await;
        assert!(verdict.skipped && !verdict.passed, "{verdict:?}");
        assert_eq!(verdict.reason, "4 of 4 citations could not be looked up");
    }
}
