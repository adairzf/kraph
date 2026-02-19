//! Generic model client: supports Ollama, DeepSeek, OpenAI, and any OpenAI-compatible API.

use crate::model_config::{ModelConfig, ModelProvider};
use crate::ollama::{ExtractedData, FusedKnowledge};
use serde_json::json;
use std::time::Duration;

/// Minimum output token budget for entity extraction.
/// Extraction produces large JSON objects; too small a budget causes truncated, unparseable output.
const EXTRACT_MIN_TOKENS: i32 = 8192;

/// Minimum output token budget for knowledge fusion.
const FUSION_MIN_TOKENS: i32 = 8192;

/// Call the configured model for entity extraction.
pub fn call_model_extract(
    config: &ModelConfig,
    prompt: &str,
    text: &str,
) -> Result<ExtractedData, String> {
    let full_prompt = format!("{}{}", prompt, text);
    // Extraction requires at least EXTRACT_MIN_TOKENS to avoid JSON truncation
    let max_tokens = config.max_tokens.max(EXTRACT_MIN_TOKENS);

    match &config.provider {
        ModelProvider::Ollama {
            base_url,
            extract_model_name,
            ..
        } => {
            call_ollama_api(base_url, extract_model_name, &full_prompt, max_tokens)
                .and_then(|response| parse_extracted_data(&response))
        }
        ModelProvider::DeepSeek {
            api_key,
            base_url,
            model_name,
        } => {
            call_openai_compatible_api(
                base_url,
                api_key,
                model_name,
                &full_prompt,
                config.temperature,
                max_tokens,
            )
            .and_then(|(response, truncated)| {
                if truncated {
                    println!("⚠️ [extract] DeepSeek response truncated (max_tokens={} too low), attempting partial parse", max_tokens);
                }
                parse_extracted_data(&response)
            })
        }
        ModelProvider::OpenAI {
            api_key,
            base_url,
            model_name,
        } => {
            call_openai_compatible_api(
                base_url,
                api_key,
                model_name,
                &full_prompt,
                config.temperature,
                max_tokens,
            )
            .and_then(|(response, truncated)| {
                if truncated {
                    println!("⚠️ [extract] API response truncated (max_tokens={} too low), attempting partial parse", max_tokens);
                }
                parse_extracted_data(&response)
            })
        }
    }
}

/// Call the configured model for knowledge fusion.
pub fn call_model_fusion(
    config: &ModelConfig,
    prompt: &str,
    historical_memories: &[String],
    new_memory: &str,
) -> Result<FusedKnowledge, String> {
    let historical_text = if historical_memories.is_empty() {
        "(no historical memories)".to_string()
    } else {
        historical_memories
            .iter()
            .enumerate()
            .map(|(i, m)| format!("{}. {}", i + 1, m))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let full_prompt = format!("{}{}\n\nNew memory:\n{}", prompt, historical_text, new_memory);
    let max_tokens = config.max_tokens.max(FUSION_MIN_TOKENS);

    match &config.provider {
        ModelProvider::Ollama {
            base_url,
            model_name,
            ..
        } => {
            call_ollama_api(base_url, model_name, &full_prompt, max_tokens)
                .and_then(|response| parse_fused_knowledge(&response))
        }
        ModelProvider::DeepSeek {
            api_key,
            base_url,
            model_name,
        } => {
            call_openai_compatible_api(
                base_url,
                api_key,
                model_name,
                &full_prompt,
                config.temperature,
                max_tokens,
            )
            .and_then(|(response, truncated)| {
                if truncated {
                    println!("⚠️ [fusion] DeepSeek response truncated, attempting partial parse");
                }
                parse_fused_knowledge(&response)
            })
        }
        ModelProvider::OpenAI {
            api_key,
            base_url,
            model_name,
        } => {
            call_openai_compatible_api(
                base_url,
                api_key,
                model_name,
                &full_prompt,
                config.temperature,
                max_tokens,
            )
            .and_then(|(response, truncated)| {
                if truncated {
                    println!("⚠️ [fusion] API response truncated, attempting partial parse");
                }
                parse_fused_knowledge(&response)
            })
        }
    }
}

/// Call the configured model for a simple single-turn prompt.
pub fn call_model_simple(
    config: &ModelConfig,
    prompt: &str,
) -> Result<String, String> {
    match &config.provider {
        ModelProvider::Ollama {
            base_url,
            model_name,
            ..
        } => call_ollama_api(base_url, model_name, prompt, config.max_tokens),
        ModelProvider::DeepSeek {
            api_key,
            base_url,
            model_name,
        } => call_openai_compatible_api(
            base_url,
            api_key,
            model_name,
            prompt,
            config.temperature,
            config.max_tokens,
        )
        .map(|(response, _)| response),
        ModelProvider::OpenAI {
            api_key,
            base_url,
            model_name,
        } => call_openai_compatible_api(
            base_url,
            api_key,
            model_name,
            prompt,
            config.temperature,
            config.max_tokens,
        )
        .map(|(response, _)| response),
    }
}

/// Call the Ollama `/api/generate` endpoint (blocking).
fn call_ollama_api(
    base_url: &str,
    model: &str,
    prompt: &str,
    max_tokens: i32,
) -> Result<String, String> {
    let url = format!("{}/api/generate", base_url.trim_end_matches('/'));
    let body = json!({
        "model": model,
        "prompt": prompt,
        "stream": false,
        "options": { "temperature": 0.2, "num_predict": max_tokens }
    });

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;

    let res = client
        .post(&url)
        .json(&body)
        .send()
        .map_err(|e| format!("Ollama request failed: {}", e))?;

    if !res.status().is_success() {
        let status = res.status();
        let err_body = res.text().unwrap_or_default();
        return Err(format!("Ollama error {}: {}", status, err_body));
    }

    let json: serde_json::Value = res.json()
        .map_err(|e| format!("Failed to parse Ollama response: {}", e))?;
    let response_text = json
        .get("response")
        .and_then(|v| v.as_str())
        .ok_or("Ollama response missing 'response' field")?
        .trim()
        .to_string();
    Ok(response_text)
}

/// Call an OpenAI-compatible chat completions API (DeepSeek, OpenAI, etc.).
/// Returns `(response_text, truncated)` where `truncated` is true when `finish_reason == "length"`.
fn call_openai_compatible_api(
    base_url: &str,
    api_key: &str,
    model: &str,
    prompt: &str,
    temperature: f32,
    max_tokens: i32,
) -> Result<(String, bool), String> {
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let body = json!({
        "model": model,
        "messages": [
            {
                "role": "user",
                "content": prompt
            }
        ],
        "temperature": temperature,
        "max_tokens": max_tokens
    });

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(300))
        .build()
        .map_err(|e| e.to_string())?;

    let res = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .map_err(|e| format!("API request failed: {}", e))?;

    if !res.status().is_success() {
        let status = res.status();
        let err_body = res.text().unwrap_or_default();
        return Err(format!("API error {}: {}", status, err_body));
    }

    let json: serde_json::Value = res.json()
        .map_err(|e| format!("Failed to parse API response: {}", e))?;

    let choice = json
        .get("choices")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .ok_or("API response missing 'choices'")?;

    // Detect truncation due to max_tokens limit
    let truncated = choice
        .get("finish_reason")
        .and_then(|r| r.as_str())
        .map_or(false, |r| r == "length");

    let response_text = choice
        .get("message")
        .and_then(|msg| msg.get("content"))
        .and_then(|content| content.as_str())
        .ok_or("API response missing 'content'")?
        .trim()
        .to_string();

    Ok((response_text, truncated))
}

/// Extract JSON from the model response and parse it as `ExtractedData`.
/// Attempts to repair truncated JSON before giving up.
fn parse_extracted_data(response: &str) -> Result<ExtractedData, String> {
    let json_str = extract_json_from_response(response)
        .ok_or("Could not find JSON in response")?;

    // Try full parse first
    if let Ok(data) = serde_json::from_str::<ExtractedData>(json_str) {
        return Ok(data);
    }

    // Attempt to repair a truncated JSON string and retry
    if let Some(repaired) = repair_truncated_json(json_str) {
        if let Ok(data) = serde_json::from_str::<ExtractedData>(&repaired) {
            println!("⚠️ [extract] Incomplete JSON auto-repaired; using partial extraction results");
            return Ok(data);
        }
    }

    Err(format!(
        "Failed to parse entity data: JSON may be truncated (response length: {} chars). \
        Try increasing 'Max Tokens' in Settings (e.g. 8192 or higher).",
        response.len()
    ))
}

/// Extract JSON from the model response and parse it as `FusedKnowledge`.
/// Attempts to repair truncated JSON before giving up.
fn parse_fused_knowledge(response: &str) -> Result<FusedKnowledge, String> {
    let json_str = extract_json_from_response(response)
        .ok_or("Could not find JSON in response")?;

    if let Ok(data) = serde_json::from_str::<FusedKnowledge>(json_str) {
        return Ok(data);
    }

    if let Some(repaired) = repair_truncated_json(json_str) {
        if let Ok(data) = serde_json::from_str::<FusedKnowledge>(&repaired) {
            println!("⚠️ [fusion] Incomplete JSON auto-repaired; using partial fusion results");
            return Ok(data);
        }
    }

    Err(format!(
        "Failed to parse fused knowledge: JSON may be truncated (response length: {} chars).",
        response.len()
    ))
}

/// Try to repair a truncated JSON string by closing unclosed brackets/braces.
///
/// Walks the string character-by-character tracking bracket depth and string state,
/// then appends the missing closing characters. Falls back to the last "safe" boundary
/// if bracket-patching alone is insufficient.
fn repair_truncated_json(s: &str) -> Option<String> {
    let mut result = s.to_string();
    let mut depth_square: i32 = 0;
    let mut depth_curly: i32 = 0;
    let mut in_string = false;
    let mut escaped = false;
    let mut last_safe_end = 0usize; // last position where both depths were 0

    for (i, ch) in result.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if in_string {
            if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '[' => depth_square += 1,
            ']' => {
                depth_square -= 1;
                if depth_square == 0 && depth_curly == 0 {
                    last_safe_end = i + 1;
                }
            }
            '{' => depth_curly += 1,
            '}' => {
                depth_curly -= 1;
                if depth_square == 0 && depth_curly == 0 {
                    last_safe_end = i + 1;
                }
            }
            _ => {}
        }
    }

    // Close an unterminated string literal
    if in_string {
        result.push('"');
    }

    // Append missing closing brackets/braces (innermost first)
    for _ in 0..depth_square.max(0) {
        result.push(']');
    }
    for _ in 0..depth_curly.max(0) {
        result.push('}');
    }

    // If patching still won't parse, the caller can try truncating to last_safe_end
    if last_safe_end > 0 && last_safe_end < s.len() {
        Some(result)
    } else if depth_square == 0 && depth_curly == 0 {
        // Already balanced — nothing to repair
        None
    } else {
        Some(result)
    }
}

/// Strip markdown code fences and extract the raw JSON content from a model response.
fn extract_json_from_response(response: &str) -> Option<&str> {
    let s = response.trim();
    let s = s
        .strip_prefix("```json")
        .or_else(|| s.strip_prefix("```"))
        .unwrap_or(s);
    let s = s.trim_end().strip_suffix("```").unwrap_or(s);
    Some(s.trim())
}
