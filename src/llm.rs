//! Optional AI: bring your own OpenAI-compatible endpoint (OpenAI, OpenRouter, Ollama, LM Studio, vLLM, ...).
//! The key lives in Windows Credential Manager (or env vars) and is never printed, logged or written to project files.
use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::time::Duration;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AiConfig {
    /// e.g. "https://api.openai.com/v1" or "http://localhost:11434/v1".
    pub base_url: String,
    /// Empty for local servers that need no key.
    pub key: String,
    pub model: String,
}

const SERVICE: &str = "visualiser";
const ACCOUNT: &str = "ai";
const DEFAULT_URL: &str = "https://api.openai.com/v1";

/// The chat-completions URL; accepts a base URL or the full endpoint.
pub fn endpoint(c: &AiConfig) -> String {
    let base = c.base_url.trim().trim_end_matches('/');
    if base.ends_with("/chat/completions") { base.into() } else { format!("{base}/chat/completions") }
}

/// Credential Manager first, else env vars (VISUALISER_AI_MODEL plus OPENAI_API_KEY and/or OPENAI_BASE_URL).
/// VISUALISER_AI_MODEL always overrides the model. Stored settings are never mixed with another
/// service's env URL/key (a global OPENAI_API_KEY must not redirect a saved Ollama setup).
pub fn load() -> Option<AiConfig> {
    let env = |k| std::env::var(k).ok().filter(|v: &String| !v.trim().is_empty());
    let model = env("VISUALISER_AI_MODEL");
    let stored = keyring::Entry::new(SERVICE, ACCOUNT).ok().and_then(|e| e.get_password().ok());
    if let Some(s) = stored {
        match serde_json::from_str::<AiConfig>(&s) {
            Ok(mut c) => {
                c.model = model.unwrap_or(c.model);
                return Some(c);
            }
            Err(_) => {
                static ONCE: std::sync::Once = std::sync::Once::new();
                ONCE.call_once(|| eprintln!("warning: stored AI settings are from an older version; run visualiser --setup-ai again"));
            }
        }
    }
    if env("OPENAI_API_KEY").is_none() && env("OPENAI_BASE_URL").is_none() {
        return None;
    }
    let base_url = env("OPENAI_BASE_URL").unwrap_or_else(|| DEFAULT_URL.into());
    if model.is_none() {
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| eprintln!("warning: OPENAI_API_KEY/OPENAI_BASE_URL is set but VISUALISER_AI_MODEL isn't; set it to use the env vars"));
    }
    Some(AiConfig { base_url, key: env("OPENAI_API_KEY").unwrap_or_default(), model: model? })
}

fn save(c: &AiConfig) -> Result<()> {
    keyring::Entry::new(SERVICE, ACCOUNT)?.set_password(&serde_json::to_string(c)?)?;
    Ok(())
}

pub fn forget() -> Result<()> {
    match keyring::Entry::new(SERVICE, ACCOUNT)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// A non-200 reply; `msg` is the API's message (never the key).
#[derive(Debug)]
struct ApiError {
    status: u16,
    msg: String,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "AI service returned {}: {}", self.status, self.msg)
    }
}

impl std::error::Error for ApiError {}

// ponytail: process-wide, assumes one AI service per run (true for the CLI and --setup-ai).
/// The server wants `max_completion_tokens` instead of `max_tokens` (OpenAI reasoning models).
static COMPLETION_TOKENS: AtomicBool = AtomicBool::new(false);
/// The server rejected structured output and answered the tool call.
static USE_TOOLS: AtomicBool = AtomicBool::new(false);

/// Renames `max_tokens` to `max_completion_tokens`; false when there was none.
fn completion_tokens(body: &mut Value) -> bool {
    match body.as_object_mut().and_then(|o| o.remove("max_tokens")) {
        Some(n) => {
            body["max_completion_tokens"] = n;
            true
        }
        None => false,
    }
}

/// POST a chat-completions request; returns the JSON body or an `ApiError`.
fn post(c: &AiConfig, body: &Value) -> Result<Value> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(600))) // --author asks for up to 8k tokens; local CPU models are slow
        .http_status_as_error(false)
        .build()
        .into();
    let mut body = body.clone();
    if COMPLETION_TOKENS.load(Relaxed) {
        completion_tokens(&mut body);
    }
    loop {
        let mut req = agent.post(&endpoint(c));
        if !c.key.trim().is_empty() {
            req = req.header("Authorization", format!("Bearer {}", c.key.trim()));
        }
        let mut resp = req.send_json(&body).map_err(|e| anyhow!("AI request failed: {e}"))?;
        let status = resp.status().as_u16();
        let text = resp.body_mut().read_to_string().unwrap_or_default();
        let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
        if status == 200 {
            return Ok(v);
        }
        // Non-JSON errors (e.g. a plain 404 from a base URL missing /v1) are shown raw.
        let raw: String = text.trim().chars().take(200).collect();
        let msg = v.pointer("/error/message").or(v.pointer("/message")).or(v.pointer("/error")).and_then(Value::as_str).unwrap_or(&raw);
        let msg = if msg.is_empty() { "no details" } else { msg };
        // OpenAI reasoning models reject `max_tokens`; local servers only know `max_tokens`.
        // Remembered, so only the first request to such a model is wasted.
        if status == 400 && msg.contains("max_completion_tokens") && completion_tokens(&mut body) {
            COMPLETION_TOKENS.store(true, Relaxed);
            continue;
        }
        return Err(ApiError { status, msg: msg.into() }.into());
    }
}

/// One JSON answer matching `schema`. First as a strict structured output (`response_format`
/// json_schema: the server constrains decoding to the schema); servers that reject that (400/422)
/// get the same request as a tool call (remembered once that works). Callers still validate the result.
pub fn call_tool(c: &AiConfig, system: &str, user: &str, tool: &str, description: &str, schema: Value, max_tokens: u32) -> Result<Value> {
    let body = json!({
        "model": c.model,
        "max_tokens": max_tokens,
        "messages": [{"role": "system", "content": system}, {"role": "user", "content": user}],
        "response_format": {"type": "json_schema", "json_schema": {"name": tool, "description": description, "strict": true, "schema": strict(schema.clone())}},
    });
    let first = if USE_TOOLS.load(Relaxed) {
        None
    } else {
        match post(c, &body) {
            Err(e) if e.downcast_ref::<ApiError>().is_some_and(|e| matches!(e.status, 400 | 422)) => Some(e),
            r => return tool_input(&r?, tool),
        }
    };
    // `tool_choice` isn't forced (Ollama ignores it, some models reject it), so the system prompt asks.
    let body = json!({
        "model": c.model,
        "max_tokens": max_tokens,
        "messages": [
            {"role": "system", "content": format!("{system}\n\nAlways answer by calling the {tool} tool.")},
            {"role": "user", "content": user},
        ],
        "tools": [{"type": "function", "function": {"name": tool, "description": description, "parameters": schema}}],
    });
    // Keep the first error too: the 400 may not have been about structured output at all
    // (which is also why only a working tool call is remembered).
    let resp = post(c, &body).map_err(|e| match &first {
        Some(f) => anyhow!("{e:#} (structured output attempt: {f:#})"),
        None => e,
    })?;
    USE_TOOLS.store(true, Relaxed);
    tool_input(&resp, tool)
}

/// `call_tool` up to 3 times: `check` validates each answer (printed as `label`), and its error
/// is fed back to the next attempt.
#[allow(clippy::too_many_arguments)]
pub fn attempts<T>(
    c: &AiConfig,
    system: &str,
    user: &str,
    tool: &str,
    description: &str,
    schema: &Value,
    max_tokens: u32,
    label: &str,
    mut check: impl FnMut(&Value) -> Result<T>,
) -> Result<T> {
    let mut user = user.to_string();
    for attempt in 1..=3 {
        eprintln!("{label} (attempt {attempt}/3) ...");
        let v = call_tool(c, system, &user, tool, description, schema.clone(), max_tokens)?;
        match check(&v) {
            Ok(t) => return Ok(t),
            Err(e) => {
                eprintln!("  rejected: {e:#}");
                user += &format!("\n\nYour previous attempt failed: {e:#}. Fix it and resubmit.");
            }
        }
    }
    bail!("the AI did not produce a working result in 3 attempts")
}

/// The schema in the form strict mode accepts: every object closed (`additionalProperties: false`)
/// with all its properties required, and the keywords some providers reject removed (callers
/// validate those limits in code anyway).
fn strict(mut s: Value) -> Value {
    if let Some(o) = s.as_object_mut() {
        for k in ["pattern", "format", "minItems", "maxItems", "minLength", "maxLength", "minimum", "maximum"] {
            o.remove(k);
        }
        if let Some(props) = o.get_mut("properties").and_then(Value::as_object_mut) {
            let keys: Vec<Value> = props.keys().map(|k| json!(k)).collect();
            for v in props.values_mut() {
                *v = strict(v.take());
            }
            o.insert("required".into(), Value::Array(keys));
            o.insert("additionalProperties".into(), json!(false));
        }
        if let Some(items) = o.get_mut("items") {
            *items = strict(items.take());
        }
    }
    s
}

/// The arguments of the first call to `tool`. Falls back to a JSON object in the reply text,
/// for models that answer in plain text instead of calling the tool.
pub fn tool_input(resp: &Value, tool: &str) -> Result<Value> {
    let msg = &resp["choices"][0]["message"];
    let call = msg["tool_calls"].as_array().and_then(|a| a.iter().find(|t| t["function"]["name"] == tool));
    if let Some(args) = call.map(|t| &t["function"]["arguments"]) {
        // Arguments are a JSON string per the spec; some servers send an object.
        return match args.as_str() {
            Some(s) => serde_json::from_str(s).context("AI tool call had malformed arguments"),
            None => Ok(args.clone()),
        };
    }
    let text = msg["content"].as_str().unwrap_or("");
    // Strict structured replies are pure JSON.
    if let Some(v) = serde_json::from_str::<Value>(text.trim()).ok().filter(Value::is_object) {
        return Ok(v);
    }
    // Reasoning goes first; then first '{' to last '}' skips fences and prose around the object.
    let text = text.rfind("</think>").map_or(text, |i| &text[i + "</think>".len()..]);
    let json = text.find('{').zip(text.rfind('}')).and_then(|(a, b)| text.get(a..=b));
    json.and_then(|j| serde_json::from_str::<Value>(j).ok()).filter(Value::is_object).context("AI reply had no tool call")
}

/// Interactive: base URL, key (hidden, optional), model; test them, store them.
pub fn setup() -> Result<()> {
    use inquire::{Password, PasswordDisplayMode, Text};
    println!("AI features let you describe the visuals you want in words.\nAny OpenAI-compatible service works (OpenAI, OpenRouter, Ollama, LM Studio, ...); the model must support structured output or tool calling.\nYour key is stored in Windows Credential Manager and only sent to that service.\n");
    let base_url = Text::new("API base URL:")
        .with_default(DEFAULT_URL)
        .with_validator(|s: &str| {
            Ok(if s.trim().starts_with("http://") || s.trim().starts_with("https://") {
                inquire::validator::Validation::Valid
            } else {
                inquire::validator::Validation::Invalid("start with http:// or https://".into())
            })
        })
        .prompt()?
        .trim()
        .to_string();
    let key = Password::new("API key (Enter for none, e.g. a local server):").with_display_mode(PasswordDisplayMode::Masked).without_confirmation().prompt()?;
    let model = Text::new("Model id:").with_validator(inquire::required!("a model id is needed")).prompt()?.trim().to_string();
    let c = AiConfig { base_url, key: key.trim().to_string(), model };
    print!("Testing {} with {} ... ", endpoint(&c), c.model);
    std::io::Write::flush(&mut std::io::stdout())?;
    // Same path as the real features, so a model without structured output or tool calling fails here, not later.
    let schema = json!({"type": "object", "properties": {"ok": {"type": "boolean"}}, "required": ["ok"]});
    call_tool(&c, "You are testing a connection.", "Reply with ok = true.", "submit", "Submit the answer.", schema, 2000)?;
    println!("works.");
    save(&c)?;
    println!("Saved. In guided mode you'll now be asked to describe the vibe; on the command line use --vibe \"...\".");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_and_tool_parsing() {
        let c = |u: &str| AiConfig { base_url: u.into(), key: String::new(), model: "m".into() };
        assert_eq!(endpoint(&c("https://api.openai.com/v1/")), "https://api.openai.com/v1/chat/completions");
        assert_eq!(endpoint(&c("http://localhost:11434/v1/chat/completions")), "http://localhost:11434/v1/chat/completions");
        let call = |args: Value| json!({"choices": [{"message": {"tool_calls": [
            {"type": "function", "function": {"name": "other", "arguments": "{}"}},
            {"type": "function", "function": {"name": "submit", "arguments": args}},
        ]}}]});
        assert_eq!(tool_input(&call(json!("{\"x\": 1}")), "submit").unwrap(), json!({"x": 1}));
        assert_eq!(tool_input(&call(json!({"x": 1})), "submit").unwrap(), json!({"x": 1}));
        let text = json!({"choices": [{"message": {"content": "<think>use {\"x\"}?</think>Here:\n```json\n{\"x\": {\"y\": 1}}\n```"}}]});
        assert_eq!(tool_input(&text, "submit").unwrap(), json!({"x": {"y": 1}}));
        assert!(tool_input(&json!({"choices": [{"message": {"content": "hi"}}]}), "submit").is_err());
        let s = strict(json!({"type": "object", "properties": {
            "id": {"type": "string", "pattern": "^a$"},
            "xs": {"type": "array", "minItems": 1, "items": {"type": "object", "properties": {"k": {"type": "number"}}}},
        }, "required": ["id"]}));
        assert_eq!(s, json!({"type": "object", "additionalProperties": false, "required": ["id", "xs"], "properties": {
            "id": {"type": "string"},
            "xs": {"type": "array", "items": {"type": "object", "additionalProperties": false, "required": ["k"], "properties": {"k": {"type": "number"}}}},
        }}));
    }
}
