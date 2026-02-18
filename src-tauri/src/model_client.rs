//! 通用模型调用接口：支持Ollama、DeepSeek、OpenAI等

use crate::model_config::{ModelConfig, ModelProvider};
use crate::ollama::{ExtractedData, FusedKnowledge};
use serde_json::json;
use std::time::Duration;

/// 调用模型进行实体提取
pub fn call_model_extract(
    config: &ModelConfig,
    prompt: &str,
    text: &str,
) -> Result<ExtractedData, String> {
    let full_prompt = format!("{}{}", prompt, text);
    
    match &config.provider {
        ModelProvider::Ollama {
            base_url,
            extract_model_name,
            ..
        } => {
            call_ollama_api(base_url, extract_model_name, &full_prompt, config.max_tokens)
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
                config.max_tokens,
            )
            .and_then(|response| parse_extracted_data(&response))
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
                config.max_tokens,
            )
            .and_then(|response| parse_extracted_data(&response))
        }
    }
}

/// 调用模型进行知识融合
pub fn call_model_fusion(
    config: &ModelConfig,
    prompt: &str,
    historical_memories: &[String],
    new_memory: &str,
) -> Result<FusedKnowledge, String> {
    let historical_text = if historical_memories.is_empty() {
        "（无历史记忆）".to_string()
    } else {
        historical_memories
            .iter()
            .enumerate()
            .map(|(i, m)| format!("{}. {}", i + 1, m))
            .collect::<Vec<_>>()
            .join("\n")
    };
    
    let full_prompt = format!("{}{}\n\n新记忆：\n{}", prompt, historical_text, new_memory);
    
    match &config.provider {
        ModelProvider::Ollama {
            base_url,
            model_name,
            ..
        } => {
            call_ollama_api(base_url, model_name, &full_prompt, config.max_tokens)
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
                config.max_tokens,
            )
            .and_then(|response| parse_fused_knowledge(&response))
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
                config.max_tokens,
            )
            .and_then(|response| parse_fused_knowledge(&response))
        }
    }
}

/// 调用模型进行简单问答
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
        ),
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
        ),
    }
}

/// 调用Ollama API
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
        .map_err(|e| format!("Ollama请求失败: {}", e))?;
        
    if !res.status().is_success() {
        let status = res.status();
        let err_body = res.text().unwrap_or_default();
        return Err(format!("Ollama错误 {}: {}", status, err_body));
    }
    
    let json: serde_json::Value = res.json()
        .map_err(|e| format!("解析Ollama响应失败: {}", e))?;
    let response_text = json
        .get("response")
        .and_then(|v| v.as_str())
        .ok_or("Ollama响应缺少response字段")?
        .trim()
        .to_string();
    Ok(response_text)
}

/// 调用OpenAI兼容API（DeepSeek、OpenAI等）
fn call_openai_compatible_api(
    base_url: &str,
    api_key: &str,
    model: &str,
    prompt: &str,
    temperature: f32,
    max_tokens: i32,
) -> Result<String, String> {
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
        .map_err(|e| format!("API请求失败: {}", e))?;
        
    if !res.status().is_success() {
        let status = res.status();
        let err_body = res.text().unwrap_or_default();
        return Err(format!("API错误 {}: {}", status, err_body));
    }
    
    let json: serde_json::Value = res.json()
        .map_err(|e| format!("解析API响应失败: {}", e))?;
    let response_text = json
        .get("choices")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|msg| msg.get("content"))
        .and_then(|content| content.as_str())
        .ok_or("API响应格式错误")?
        .trim()
        .to_string();
    Ok(response_text)
}

/// 从响应中提取JSON并解析为ExtractedData
fn parse_extracted_data(response: &str) -> Result<ExtractedData, String> {
    let json_str = extract_json_from_response(response)
        .ok_or("无法从响应中提取JSON")?;
    let data: ExtractedData = serde_json::from_str(json_str)
        .map_err(|e| format!("解析实体数据失败: {}", e))?;
    Ok(data)
}

/// 从响应中提取JSON并解析为FusedKnowledge
fn parse_fused_knowledge(response: &str) -> Result<FusedKnowledge, String> {
    let json_str = extract_json_from_response(response)
        .ok_or("无法从响应中提取JSON")?;
    let data: FusedKnowledge = serde_json::from_str(json_str)
        .map_err(|e| format!("解析融合知识失败: {}", e))?;
    Ok(data)
}

/// 从响应中提取JSON内容
fn extract_json_from_response(response: &str) -> Option<&str> {
    let s = response.trim();
    // 去掉 markdown 代码块
    let s = s
        .strip_prefix("```json")
        .or_else(|| s.strip_prefix("```"))
        .unwrap_or(s);
    let s = s.trim_end().strip_suffix("```").unwrap_or(s);
    Some(s.trim())
}
