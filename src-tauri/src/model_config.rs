//! 模型配置模块：支持本地Ollama和在线API（DeepSeek等）

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ModelProvider {
    #[serde(rename = "ollama")]
    Ollama {
        base_url: String,
        model_name: String,
        extract_model_name: String,
    },
    #[serde(rename = "deepseek")]
    DeepSeek {
        api_key: String,
        base_url: String,
        model_name: String,
    },
    #[serde(rename = "openai")]
    OpenAI {
        api_key: String,
        base_url: String,
        model_name: String,
    },
}

impl Default for ModelProvider {
    fn default() -> Self {
        ModelProvider::Ollama {
            base_url: "http://localhost:11434".to_string(),
            model_name: "qwen2.5:7b".to_string(),
            extract_model_name: "qwen2.5:7b".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub provider: ModelProvider,
    pub temperature: f32,
    pub max_tokens: i32,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            provider: ModelProvider::default(),
            temperature: 0.2,
            max_tokens: 4096,
        }
    }
}

impl ModelConfig {
    pub fn load_from_file(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(path)
            .map_err(|e| format!("读取配置文件失败: {}", e))?;
        let config: ModelConfig = serde_json::from_str(&content)
            .map_err(|e| format!("解析配置文件失败: {}", e))?;
        Ok(config)
    }

    pub fn save_to_file(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("创建配置目录失败: {}", e))?;
        }
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("序列化配置失败: {}", e))?;
        fs::write(path, content)
            .map_err(|e| format!("写入配置文件失败: {}", e))?;
        Ok(())
    }
}
