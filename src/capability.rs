/// What a model does with `temperature` while it is thinking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThinkingTemperature {
    /// Temperature still changes the answer while the model thinks. Send it.
    Honored,
    /// Accepted without an error but has no effect. Not sent; the record says so.
    Ignored,
    /// Sending it fails the request (HTTP 400). Never sent.
    Rejected,
    /// The model has no thinking mode, so temperature is always live. Send it.
    NoThinking,
}

impl ThinkingTemperature {
    /// The stable string used in records: `honored`, `ignored`, `rejected`, `no_thinking`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Honored => "honored",
            Self::Ignored => "ignored",
            Self::Rejected => "rejected",
            Self::NoThinking => "no_thinking",
        }
    }
}

/// A class plus where it came from: a documentation URL, or [`UNKNOWN_SOURCE`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capability {
    pub thinking_temperature: ThinkingTemperature,
    pub source: &'static str,
}

/// The label for an endpoint no documentation page covers.
pub const UNKNOWN_SOURCE: &str = "unknown, endpoint's own behaviour";

/// A model the caller controls end to end, including its thinking switch.
/// Only `deepseek-flash` is in the table; any other id resolves to it.
pub fn builtin_capability(model: &str) -> Capability {
    match model {
        "deepseek-flash" => Capability {
            thinking_temperature: ThinkingTemperature::Ignored,
            source: "https://api-docs.deepseek.com/guides/thinking_mode/",
        },
        _ => builtin_capability("deepseek-flash"),
    }
}

/// A user-supplied OpenAI-compatible endpoint, judged by its host and model name.
///
/// No thinking switch is sent to such an endpoint, so the model does what it does by default. A host
/// no page covers is `Honored` with [`UNKNOWN_SOURCE`].
pub fn endpoint_capability(base_url: &str, model: &str) -> Capability {
    let host = base_host(base_url);
    let m = model.trim().to_ascii_lowercase();
    let cap = |t, source| Capability {
        thinking_temperature: t,
        source,
    };
    match host.as_str() {
        "api.deepseek.com" => cap(
            ThinkingTemperature::Ignored,
            "https://api-docs.deepseek.com/guides/thinking_mode/",
        ),
        "api.openai.com" => {
            let t = if is_openai_reasoning(&m) {
                ThinkingTemperature::Rejected
            } else {
                ThinkingTemperature::NoThinking
            };
            cap(t, "https://platform.openai.com/docs/guides/reasoning")
        }
        // Newer Claude models return 400 for some or all temperature values, and leaving it out is
        // never an error, so it is never sent.
        "api.anthropic.com" => cap(
            ThinkingTemperature::Rejected,
            "https://platform.claude.com/docs/en/cli-sdks-libraries/libraries/openai-sdk",
        ),
        "api.groq.com" => cap(
            ThinkingTemperature::Honored,
            "https://console.groq.com/docs/reasoning",
        ),
        "generativelanguage.googleapis.com" => {
            if is_gemini_ignoring(&m) {
                cap(
                    ThinkingTemperature::Ignored,
                    "https://ai.google.dev/gemini-api/docs/troubleshooting",
                )
            } else {
                cap(
                    ThinkingTemperature::Honored,
                    "https://ai.google.dev/gemini-api/docs/gemini-3",
                )
            }
        }
        // A router drops parameters an upstream does not support, so a model that would 400 on its
        // own API has the parameter dropped here instead.
        "openrouter.ai" => {
            let (vendor, name) = m.split_once('/').unwrap_or(("", m.as_str()));
            let drops = match vendor {
                "openai" => is_openai_reasoning(name),
                "anthropic" | "deepseek" => true,
                "google" => is_gemini_ignoring(name),
                _ => false,
            };
            if drops {
                cap(
                    ThinkingTemperature::Ignored,
                    "https://openrouter.ai/docs/api_reference/parameters",
                )
            } else {
                cap(ThinkingTemperature::Honored, UNKNOWN_SOURCE)
            }
        }
        _ => cap(ThinkingTemperature::Honored, UNKNOWN_SOURCE),
    }
}

fn base_host(base: &str) -> String {
    let rest = base.split_once("://").map(|(_, r)| r).unwrap_or(base);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let host = if let Some(v6) = authority.strip_prefix('[') {
        v6.split(']').next().unwrap_or("")
    } else {
        authority.split(':').next().unwrap_or("")
    };
    host.trim_end_matches('.').to_ascii_lowercase()
}

/// o1 / o3 / o4-mini / gpt-5*: OpenAI's reasoning models. `omni-...` and a bare `o` are not.
fn is_openai_reasoning(m: &str) -> bool {
    let o_series = m.strip_prefix('o').is_some_and(|rest| {
        let tail = rest.trim_start_matches(|c: char| c.is_ascii_digit());
        tail.len() < rest.len() && (tail.is_empty() || tail.starts_with('-'))
    });
    o_series || m.starts_with("gpt-5")
}

/// Gemini 3.6 and later: sampling parameters are ignored if set.
fn is_gemini_ignoring(m: &str) -> bool {
    let Some(rest) = m.strip_prefix("gemini-") else {
        return false;
    };
    let ver: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    let mut parts = ver.split('.');
    let major: u32 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let minor: u32 = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    major >= 4 || (major == 3 && minor >= 6)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ThinkingTemperature::*;

    fn tt(base: &str, model: &str) -> ThinkingTemperature {
        endpoint_capability(base, model).thinking_temperature
    }

    #[test]
    fn rejected_cases() {
        for m in [
            "o1",
            "o3",
            "o3-mini",
            "o4-mini",
            "gpt-5",
            "gpt-5-mini",
            "gpt-5.2",
            "O3",
        ] {
            assert_eq!(tt("https://api.openai.com/v1", m), Rejected, "{m}");
        }
        assert_eq!(
            tt("https://api.anthropic.com/v1", "claude-sonnet-4-5"),
            Rejected
        );
    }

    #[test]
    fn ignored_cases() {
        assert_eq!(
            builtin_capability("deepseek-flash").thinking_temperature,
            Ignored
        );
        assert_eq!(tt("https://api.deepseek.com/v1", "deepseek-flash"), Ignored);
        let or = "https://openrouter.ai/api/v1";
        for m in [
            "openai/o3",
            "openai/gpt-5",
            "anthropic/claude-opus-4.7",
            "google/gemini-3.6-flash",
        ] {
            assert_eq!(tt(or, m), Ignored, "{m}");
        }
        let g = "https://generativelanguage.googleapis.com/v1beta/openai";
        assert_eq!(tt(g, "gemini-3.6-flash"), Ignored);
        assert_eq!(tt(g, "gemini-4-pro"), Ignored);
    }

    #[test]
    fn honored_cases_and_unknown_is_labelled() {
        let g = "https://generativelanguage.googleapis.com/v1beta/openai";
        assert_eq!(tt(g, "gemini-2.5-flash"), Honored);
        assert_eq!(tt(g, "gemini-3-pro"), Honored);
        assert_eq!(
            tt("https://api.groq.com/openai/v1", "qwen/qwen3-32b"),
            Honored
        );
        for (b, m) in [
            ("http://127.0.0.1:11434/v1", "deepseek-r1"),
            ("http://127.0.0.1:1234/v1", "qwen3"),
            ("https://api.together.xyz/v1", "x"),
            ("https://openrouter.ai/api/v1", "meta-llama/llama-4"),
        ] {
            let c = endpoint_capability(b, m);
            assert_eq!(c.thinking_temperature, Honored, "{b} {m}");
            assert_eq!(c.source, UNKNOWN_SOURCE, "{b} {m}");
        }
    }

    #[test]
    fn no_thinking_cases_with_negative_controls_for_the_o_series_matcher() {
        for m in ["gpt-4.1", "gpt-4o", "omni-moderation-latest", "o", "ox-1"] {
            assert_eq!(tt("https://api.openai.com/v1", m), NoThinking, "{m}");
        }
    }

    #[test]
    fn every_documented_capability_names_its_source() {
        for (b, m) in [
            ("https://api.openai.com/v1", "o3"),
            ("https://api.openai.com/v1", "gpt-4o"),
            ("https://api.anthropic.com/v1", "claude"),
            ("https://api.deepseek.com/v1", "deepseek-flash"),
            ("https://api.groq.com/openai/v1", "x"),
            ("https://openrouter.ai/api/v1", "openai/o3"),
        ] {
            assert!(
                endpoint_capability(b, m).source.starts_with("https://"),
                "{b} {m}"
            );
        }
        assert_eq!(NoThinking.as_str(), "no_thinking");
    }

    #[test]
    fn host_parsing_ignores_port_case_path_and_trailing_dot() {
        assert_eq!(
            base_host("HTTPS://API.OpenAI.com.:443/v1?x=1"),
            "api.openai.com"
        );
        assert_eq!(base_host("http://[::1]:11434/v1"), "::1");
    }
}
