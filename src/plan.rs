use crate::capability::{
    builtin_capability, endpoint_capability, ThinkingTemperature as T, ThinkingTemperature,
};

/// Where the request is going.
#[derive(Debug, Clone, Copy)]
pub enum Target<'a> {
    /// A model whose thinking switch the caller controls. `thinking` says whether it is on.
    Builtin { thinking: bool },
    /// A user-supplied endpoint (its base URL). No thinking switch is sent, so a thinking model is
    /// treated as thinking whatever the caller's setting says.
    Endpoint { base_url: &'a str },
}

/// What happens to `temperature` on one request, and why.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TemperaturePlan {
    /// Whether `temperature` goes in the request body.
    pub send: bool,
    /// `ignored_while_thinking` or `rejected_while_thinking` when left out; `None` when sent.
    pub not_applied: Option<&'static str>,
    pub thinking_temperature: ThinkingTemperature,
    pub source: &'static str,
}

impl TemperaturePlan {
    /// The value to put in the request body, or `None` to leave the field out.
    pub fn apply(&self, requested: f64) -> Option<f64> {
        self.send.then_some(requested)
    }

    /// The record to store with the response, so a reader can see when the setting did nothing.
    /// Keys: `send`, `not_applied` (only when left out), `thinking_temperature`, `source`, `requested`.
    pub fn to_json(&self, requested: f64) -> String {
        let mut s = format!("{{\"send\":{}", self.send);
        if let Some(n) = self.not_applied {
            s.push_str(&format!(",\"not_applied\":{}", quote(n)));
        }
        s.push_str(&format!(
            ",\"thinking_temperature\":{},\"source\":{},\"requested\":{}}}",
            quote(self.thinking_temperature.as_str()),
            quote(self.source),
            requested
        ));
        s
    }
}

fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Decide whether temperature is sent. Pure.
///
/// A model that rejects temperature while thinking would answer 400, and one that ignores it would
/// let the request claim a control that does nothing, so in both cases it is left out.
pub fn temperature_plan(model: &str, target: Target<'_>) -> TemperaturePlan {
    let (cap, thinking_on) = match target {
        Target::Builtin { thinking } => (builtin_capability(model), thinking),
        Target::Endpoint { base_url } => (endpoint_capability(base_url, model), true),
    };
    let not_applied = match (cap.thinking_temperature, thinking_on) {
        (T::Ignored, true) => Some("ignored_while_thinking"),
        (T::Rejected, true) => Some("rejected_while_thinking"),
        _ => None,
    };
    TemperaturePlan {
        send: not_applied.is_none(),
        not_applied,
        thinking_temperature: cap.thinking_temperature,
        source: cap.source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::UNKNOWN_SOURCE;

    const LOCAL: &str = "http://127.0.0.1:11434/v1";

    fn sent(model: &str, target: Target<'_>) -> bool {
        temperature_plan(model, target).apply(0.7).is_some()
    }

    #[test]
    fn a_model_that_rejects_temperature_never_receives_it() {
        for (base, model) in [
            ("https://api.openai.com/v1", "o3"),
            ("https://api.openai.com/v1", "o4-mini"),
            ("https://api.openai.com/v1", "gpt-5.2"),
            ("https://api.anthropic.com/v1", "claude-opus-4-7"),
        ] {
            let p = temperature_plan(model, Target::Endpoint { base_url: base });
            assert!(!p.send, "{base} {model}");
            assert_eq!(p.not_applied, Some("rejected_while_thinking"));
        }
    }

    #[test]
    fn a_model_that_ignores_temperature_while_thinking_is_not_sent_it_and_says_so() {
        let p = temperature_plan("deepseek-flash", Target::Builtin { thinking: true });
        assert!(!p.send);
        assert_eq!(p.not_applied, Some("ignored_while_thinking"));
        assert!(p.source.starts_with("https://api-docs.deepseek.com/"));
        // Thinking off on a model whose switch we control: the setting is live again.
        assert!(sent("deepseek-flash", Target::Builtin { thinking: false }));
        // The same model on a user endpoint: no switch is sent, so it thinks, so temperature is dead.
        assert!(!sent(
            "deepseek-flash",
            Target::Endpoint {
                base_url: "https://api.deepseek.com/v1"
            }
        ));
        assert!(!sent(
            "openai/o3",
            Target::Endpoint {
                base_url: "https://openrouter.ai/api/v1"
            }
        ));
    }

    #[test]
    fn honored_and_no_thinking_models_get_it() {
        for (base, model) in [
            (LOCAL, "qwen3"),
            ("https://api.groq.com/openai/v1", "openai/gpt-oss-120b"),
            ("https://api.openai.com/v1", "gpt-4.1"),
            ("https://openrouter.ai/api/v1", "meta-llama/llama-4"),
        ] {
            assert!(
                sent(model, Target::Endpoint { base_url: base }),
                "{base} {model}"
            );
        }
        let p = temperature_plan("qwen3", Target::Endpoint { base_url: LOCAL });
        assert_eq!(p.source, UNKNOWN_SOURCE);
        assert_eq!(p.not_applied, None);
    }

    #[test]
    fn the_record_shows_the_setting_did_nothing() {
        let p = temperature_plan(
            "o3",
            Target::Endpoint {
                base_url: "https://api.openai.com/v1",
            },
        );
        assert_eq!(
            p.to_json(0.7),
            "{\"send\":false,\"not_applied\":\"rejected_while_thinking\",\"thinking_temperature\":\"rejected\",\"source\":\"https://platform.openai.com/docs/guides/reasoning\",\"requested\":0.7}"
        );
        let q = temperature_plan("qwen3", Target::Endpoint { base_url: LOCAL });
        assert!(q
            .to_json(0.2)
            .starts_with("{\"send\":true,\"thinking_temperature\":\"honored\""));
    }

    #[test]
    fn json_strings_are_escaped() {
        assert_eq!(quote("a\"b\\c\n"), "\"a\\\"b\\\\c\\u000a\"");
    }
}
