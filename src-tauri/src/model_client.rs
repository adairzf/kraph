//! 通用模型调用接口：支持Ollama、DeepSeek、OpenAI等

use crate::model_config::{ModelConfig, ModelProvider};
use crate::ollama::{ExtractedData, FusedKnowledge};
use serde_json::json;
use std::time::Duration;

/// 实体提取专用最小输出 token 数。
/// 提取任务需要输出完整的大型 JSON，必须给足空间，否则 JSON 会被截断导致解析失败。
const EXTRACT_MIN_TOKENS: i32 = 8192;

/// 知识融合专用最小输出 token 数。
const FUSION_MIN_TOKENS: i32 = 8192;

/// 调用模型进行实体提取
pub fn call_model_extract(
    config: &ModelConfig,
    prompt: &str,
    text: &str,
) -> Result<ExtractedData, String> {
    let full_prompt = format!("{}{}", prompt, text);
    // 提取任务强制使用不低于 EXTRACT_MIN_TOKENS 的输出上限，避免 JSON 被截断
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
                    println!("⚠️ [提取] DeepSeek 响应被截断（max_tokens={} 不够），尝试解析部分结果", max_tokens);
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
                    println!("⚠️ [提取] API 响应被截断（max_tokens={} 不够），尝试解析部分结果", max_tokens);
                }
                parse_extracted_data(&response)
            })
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
                    println!("⚠️ [融合] DeepSeek 响应被截断，尝试解析部分结果");
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
                    println!("⚠️ [融合] API 响应被截断，尝试解析部分结果");
                }
                parse_fused_knowledge(&response)
            })
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
/// 返回 (响应文本, 是否被截断)
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
        .map_err(|e| format!("API请求失败: {}", e))?;
        
    if !res.status().is_success() {
        let status = res.status();
        let err_body = res.text().unwrap_or_default();
        return Err(format!("API错误 {}: {}", status, err_body));
    }
    
    let json: serde_json::Value = res.json()
        .map_err(|e| format!("解析API响应失败: {}", e))?;

    let choice = json
        .get("choices")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .ok_or("API响应格式错误：缺少choices")?;

    // 检测是否因 max_tokens 限制被截断
    let truncated = choice
        .get("finish_reason")
        .and_then(|r| r.as_str())
        .map_or(false, |r| r == "length");

    let response_text = choice
        .get("message")
        .and_then(|msg| msg.get("content"))
        .and_then(|content| content.as_str())
        .ok_or("API响应格式错误：缺少content")?
        .trim()
        .to_string();

    Ok((response_text, truncated))
}

/// 从响应中提取JSON并解析为ExtractedData，带截断修复
fn parse_extracted_data(response: &str) -> Result<ExtractedData, String> {
    let json_str = extract_json_from_response(response)
        .ok_or("无法从响应中提取JSON")?;

    // 先尝试完整解析
    if let Ok(data) = serde_json::from_str::<ExtractedData>(json_str) {
        return Ok(data);
    }

    // 完整解析失败，尝试修复被截断的 JSON 后重试
    if let Some(repaired) = repair_truncated_json(json_str) {
        if let Ok(data) = serde_json::from_str::<ExtractedData>(&repaired) {
            println!("⚠️ [提取] JSON 响应不完整，已自动修复并使用部分提取结果");
            return Ok(data);
        }
    }

    Err(format!(
        "解析实体数据失败：JSON 可能被截断（响应长度 {} 字符）。\
        建议在设置中将「最大 Tokens」调高（如 8192 或以上）",
        response.len()
    ))
}

/// 从响应中提取JSON并解析为FusedKnowledge，带截断修复
fn parse_fused_knowledge(response: &str) -> Result<FusedKnowledge, String> {
    let json_str = extract_json_from_response(response)
        .ok_or("无法从响应中提取JSON")?;

    if let Ok(data) = serde_json::from_str::<FusedKnowledge>(json_str) {
        return Ok(data);
    }

    if let Some(repaired) = repair_truncated_json(json_str) {
        if let Ok(data) = serde_json::from_str::<FusedKnowledge>(&repaired) {
            println!("⚠️ [融合] JSON 响应不完整，已自动修复并使用部分融合结果");
            return Ok(data);
        }
    }

    Err(format!(
        "解析融合知识失败：JSON 可能被截断（响应长度 {} 字符）",
        response.len()
    ))
}

/// 尝试修复被截断的 JSON 字符串：
/// 找到最后一个完整的对象/数组边界，补全剩余的括号使 JSON 合法
fn repair_truncated_json(s: &str) -> Option<String> {
    let mut result = s.to_string();
    let mut depth_square: i32 = 0;
    let mut depth_curly: i32 = 0;
    let mut in_string = false;
    let mut escaped = false;
    let mut last_safe_end = 0usize; // 上一个"安全"的截断点（depth_square==0 且 depth_curly==0）

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

    // 如果解析时还在字符串中（截断在字符串内），先关闭它
    if in_string {
        result.push('"');
        // 重新检查深度不再变化，直接补全括号
    }

    // 补全未关闭的结构（从内到外）
    for _ in 0..depth_square.max(0) {
        result.push(']');
    }
    for _ in 0..depth_curly.max(0) {
        result.push('}');
    }

    // 如果补全后解析还是失败，退而求其次：截断到最后一个安全边界
    if last_safe_end > 0 && last_safe_end < s.len() {
        // 返回两种候选，调用方可以都尝试一下
        // 这里先返回补全版本，调用方失败时可以再试截断版本
        Some(result)
    } else if depth_square == 0 && depth_curly == 0 {
        // 原本就是合法的，直接返回（说明问题不在括号上）
        None
    } else {
        Some(result)
    }
}

/// 从响应中提取JSON内容（去除 markdown 代码块包装）
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
