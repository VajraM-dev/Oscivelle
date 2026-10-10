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
        "bedrock" => (
            format!("https://bedrock-mantle.{}.api.aws/anthropic/v1/messages", c.region.as_deref().unwrap_or("us-east-1")),
            model.unwrap_or_else(|| "anthropic.claude-sonnet-5-5".into()),
        ),
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
    let mut req = agent.post(&url).header("x-api-key", &c.key).header("anthropic-version", "2023-06-01");
    if c.provider == "bedrock" {
        // Unverified which one the bedrock-mantle endpoint wants: send both.
        req = req.header("Authorization", format!("Bearer {}", c.key));
    }
    let mut resp = req
        .send_json(body)
        .map_err(|e| anyhow!("AI request failed: {e}"))?;
    let status = resp.status().as_u16();
    let v: Value = resp.body_mut().read_json().unwrap_or(Value::Null);
    if status != 200 {
        let msg = v.pointer("/error/message").or(v.pointer("/message")).and_then(Value::as_str).unwrap_or("no details");
        bail!("AI service returned {status}: {msg}");
    }
    Ok(v)
}

/// One forced tool call; returns the tool input JSON.
pub fn call_tool(c: &AiConfig, system: &str, user: &str, tool: &str, description: &str, schema: Value, max_tokens: u32) -> Result<Value> {
    let (_, model) = endpoint(c);
    let body = json!({
        "model": model,
        "max_tokens": max_tokens,
        "system": system,
        "messages": [{"role": "user", "content": user}],
        "tools": [{"name": tool, "description": description, "input_schema": schema}],
        "tool_choice": {"type": "tool", "name": tool},
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
    let c = AiConfig { provider: (if bedrock { "bedrock" } else { "anthropic" }).into(), key: key.trim().to_string(), region, model: None };
    let (_, model) = endpoint(&c);
    print!("Testing the key with {model} ... ");
    std::io::Write::flush(&mut std::io::stdout())?;
    post(&c, &json!({"model": model, "max_tokens": 8, "messages": [{"role": "user", "content": "Say ok"}]}))?;
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
        let b = AiConfig { provider: "bedrock".into(), key: "k".into(), region: Some("eu-west-1".into()), model: None };
        assert_eq!(endpoint(&b).0, "https://bedrock-mantle.eu-west-1.api.aws/anthropic/v1/messages");
        assert!(endpoint(&b).1.starts_with("anthropic.claude-"));
        let a = AiConfig { provider: "anthropic".into(), ..b };
        assert_eq!(endpoint(&a).0, "https://api.anthropic.com/v1/messages");
        let resp = json!({"content": [{"type": "text", "text": "hi"}, {"type": "tool_use", "name": "submit", "input": {"x": 1}}]});
        assert_eq!(tool_input(&resp, "submit").unwrap(), json!({"x": 1}));
        assert!(tool_input(&json!({"content": []}), "submit").is_err());
    }
}
