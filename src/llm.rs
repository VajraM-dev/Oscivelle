//! Optional AI: bring your own Anthropic or Amazon Bedrock API key. The key lives in Windows
//! Credential Manager (or env vars) and is never printed, logged or written to project files.
use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AiConfig {
    /// "anthropic" or "bedrock".
    pub provider: String,
    pub key: String,
    /// Bedrock region, e.g. "us-east-1".
    #[serde(default)]
    pub region: Option<String>,
    /// Override the default model id.
    #[serde(default)]
    pub model: Option<String>,
}

const SERVICE: &str = "visualiser";
const ACCOUNT: &str = "ai";

/// (url, model) for a config.
pub fn endpoint(c: &AiConfig) -> (String, String) {
    let model = std::env::var("VISUALISER_AI_MODEL").ok().or_else(|| c.model.clone());
    match c.provider.as_str() {
        "bedrock" => {
            let region = c.region.as_deref().unwrap_or("us-east-1");
            let model = model.unwrap_or_else(|| "anthropic.claude-sonnet-5-5".into());
            // Inference-profile ids (global./us./... or an ARN) only exist on the InvokeModel API;
            // some accounts can only use that one.
            let url = if model.starts_with("anthropic.") {
                format!("https://bedrock-mantle.{region}.api.aws/anthropic/v1/messages")
            } else {
                format!("https://bedrock-runtime.{region}.amazonaws.com/model/{}/invoke", model.replace(':', "%3A").replace('/', "%2F"))
            };
            (url, model)
        }
        _ => ("https://api.anthropic.com/v1/messages".into(), model.unwrap_or_else(|| "claude-sonnet-5-5".into())),
    }
}

/// Env vars first (ANTHROPIC_API_KEY, or AWS_BEARER_TOKEN_BEDROCK + AWS_REGION), then Credential Manager.
pub fn load() -> Option<AiConfig> {
    let env = |k| std::env::var(k).ok().filter(|v| !v.trim().is_empty());
    if let Some(key) = env("ANTHROPIC_API_KEY") {
        return Some(AiConfig { provider: "anthropic".into(), key, region: None, model: None });
    }
    if let Some(key) = env("AWS_BEARER_TOKEN_BEDROCK") {
        return Some(AiConfig { provider: "bedrock".into(), key, region: std::env::var("AWS_REGION").ok(), model: None });
    }
    let s = keyring::Entry::new(SERVICE, ACCOUNT).ok()?.get_password().ok()?;
    serde_json::from_str(&s).ok()
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

/// POST a Messages request; returns the JSON body or an error with the API's message (never the key).
fn post(c: &AiConfig, body: &Value) -> Result<Value> {
    let (url, _) = endpoint(c);
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(240))) // --author asks for up to 8k tokens
        .http_status_as_error(false)
        .build()
        .into();
    let resp = if url.contains("bedrock-runtime") {
        // InvokeModel: model in the URL, version in the body, key as a bearer token.
        let mut body = body.clone();
        body.as_object_mut().context("request body")?.remove("model");
        body["anthropic_version"] = json!("bedrock-2023-05-31");
        agent.post(&url).header("Authorization", format!("Bearer {}", c.key)).send_json(&body)
    } else {
        // Anthropic and bedrock-mantle take x-api-key; mantle rejects requests that also carry Authorization.
        agent.post(&url).header("x-api-key", &c.key).header("anthropic-version", "2023-06-01").send_json(body)
    };
    let mut resp = resp.map_err(|e| anyhow!("AI request failed: {e}"))?;
    let status = resp.status().as_u16();
    let v: Value = resp.body_mut().read_json().unwrap_or(Value::Null);
    if status != 200 {
        let msg = v.pointer("/error/message").or(v.pointer("/message")).and_then(Value::as_str).unwrap_or("no details");
        bail!("AI service returned {status}: {msg}");
    }
    Ok(v)
}

/// One tool call; returns the tool input JSON. Sonnet 5.5 rejects a forced `tool_choice`, so the
/// call is asked for in the system prompt instead (callers validate the input either way).
pub fn call_tool(c: &AiConfig, system: &str, user: &str, tool: &str, description: &str, schema: Value, max_tokens: u32) -> Result<Value> {
    let (_, model) = endpoint(c);
    let body = json!({
        "model": model,
        "max_tokens": max_tokens,
        "system": format!("{system}

Always answer by calling the {tool} tool."),
        "messages": [{"role": "user", "content": user}],
        "tools": [{"name": tool, "description": description, "input_schema": schema}],
    });
    tool_input(&post(c, &body)?, tool)
}

/// The `input` of the first tool_use block named `tool`.
pub fn tool_input(resp: &Value, tool: &str) -> Result<Value> {
    resp["content"]
        .as_array()
        .and_then(|a| a.iter().find(|b| b["type"] == "tool_use" && b["name"] == tool))
        .map(|b| b["input"].clone())
        .context("AI reply had no tool call")
}

/// Interactive: choose provider, paste key (hidden), test it, store it.
pub fn setup() -> Result<()> {
    use inquire::{Password, PasswordDisplayMode, Select, Text};
    println!("AI features let you describe the visuals you want in words.\nYour key is stored in Windows Credential Manager and only sent to the AI service.\n");
    let providers = vec!["Anthropic API key", "Amazon Bedrock API key"];
    let bedrock = Select::new("Which key do you have?", providers).raw_prompt()?.index == 1;
    let key = Password::new("Paste your key:").with_display_mode(PasswordDisplayMode::Masked).without_confirmation().prompt()?;
    let region = if bedrock { Some(Text::new("AWS region:").with_default("us-east-1").prompt()?) } else { None };
    // Bedrock accounts differ in which models (and which of its two APIs) they may use:
    // `anthropic.…` ids go to the Messages endpoint, `global.anthropic.…`-style ids to InvokeModel.
    let model = if bedrock {
        Some(Text::new("Model id (Enter for the default):").with_default("anthropic.claude-sonnet-5-5").prompt()?.trim().to_string())
    } else {
        None
    };
    let mut c = AiConfig { provider: (if bedrock { "bedrock" } else { "anthropic" }).into(), key: key.trim().to_string(), region, model };
    // On Bedrock, a model the account can't use (403/404) falls through to the ones open to every account.
    let fallbacks = if bedrock { ["anthropic.claude-sonnet-5", "anthropic.claude-haiku-4-5", "anthropic.claude-opus-4-8"].as_slice() } else { &[] };
    let mut tries = c.model.clone().into_iter().map(Some).chain(fallbacks.iter().map(|m| Some(m.to_string()))).collect::<Vec<_>>();
    if tries.is_empty() {
        tries.push(None);
    }
    let mut seen = Vec::new();
    tries.retain(|m| if seen.contains(m) { false } else { seen.push(m.clone()); true });
    let mut last = None;
    for m in tries {
        c.model = m;
        let (_, model) = endpoint(&c);
        print!("Testing the key with {model} ... ");
        std::io::Write::flush(&mut std::io::stdout())?;
        match post(&c, &json!({"model": model, "max_tokens": 64, "messages": [{"role": "user", "content": "Say ok"}]})) {
            Ok(_) => {
                println!("works.");
                last = None;
                break;
            }
            Err(e) => {
                let msg = format!("{e:#}");
                println!("no ({msg}).");
                let model_problem = msg.contains(" 403:") || msg.contains(" 404:");
                last = Some(e);
                if !bedrock || !model_problem {
                    break;
                }
            }
        }
    }
    if let Some(e) = last {
        if bedrock {
            bail!("{e:#}
No Claude model worked with this key. In the AWS console, open Amazon Bedrock > Model access for this region, enable a Claude model, and check the key's permissions.");
        }
        return Err(e);
    }
    save(&c)?;
    println!("Saved. In guided mode you'll now be asked to describe the vibe; on the command line use --vibe \"...\".");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_and_tool_parsing() {
        let b = AiConfig { provider: "bedrock".into(), key: "k".into(), region: Some("eu-west-1".into()), model: None };
        assert_eq!(endpoint(&b).0, "https://bedrock-mantle.eu-west-1.api.aws/anthropic/v1/messages");
        assert!(endpoint(&b).1.starts_with("anthropic.claude-"));
        let g = AiConfig { model: Some("global.anthropic.claude-haiku-5-5".into()), ..b.clone() };
        assert_eq!(endpoint(&g).0, "https://bedrock-runtime.eu-west-1.amazonaws.com/model/global.anthropic.claude-haiku-5-5/invoke");
        let a = AiConfig { provider: "anthropic".into(), ..b };
        assert_eq!(endpoint(&a).0, "https://api.anthropic.com/v1/messages");
        let resp = json!({"content": [{"type": "text", "text": "hi"}, {"type": "tool_use", "name": "submit", "input": {"x": 1}}]});
        assert_eq!(tool_input(&resp, "submit").unwrap(), json!({"x": 1}));
        assert!(tool_input(&json!({"content": []}), "submit").is_err());
    }
}
