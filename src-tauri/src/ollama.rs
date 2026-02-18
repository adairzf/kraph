//! Ollama API：实体提取，支持自动启动本机 Ollama 服务
//!
//! 说明：实体拆分（NER）用 encoder 小模型即可（如 BERT 做序列标注），
//! 当前通过 Ollama 用「小参数量生成模型」做抽取，以兼顾无需额外部署。
//! 若后续接入纯 NER 模型（如 HuggingFace 中文 NER），可在此处替换为本地 encoder 调用。

use serde::{Deserialize, Serialize};
use std::process::Stdio;
use std::time::Duration;

pub const ENTITY_EXTRACT_PROMPT: &str = r#"你是一个专业的命名实体识别(NER)助手。请从文本中**尽可能完整地提取**以下类型的实体和关系：

**实体类型**：
1. Person（人物）：人名、角色名
2. Organization（组织/机构）：公司、军队、部门、组织、机构、项目名称、计划名称（包括括号中的缩写和全称）
3. Location（地点）：地点、建筑、太空站、基地、星球、飞船等
4. Time（时间）：日期、时间点、年龄、纪元
5. Event（事件）：战役、任务、行动、计划、项目

**关键提取规则**：
- **括号内容必须提取**：如"深空防卫军(DSOF)"要提取为Organization，括号里的缩写也是重要信息
- **缩写也是实体**：如ICMA、HGA、SED、DSOF等都是Organization类型的实体
- **所有组织机构都要提取**：包括政府部门、军队、大学、实验室、研究院等
- **地点包括飞船和基地**：如"月球基地"、"赤霄舰"都是Location
- **时间包括多种格式**：如"地球历2067.6.16"、"UCT纪元7年"、"30岁"都是Time
- attributes存储该实体的特征（如身份、军衔、年龄、性别等），不要把特征创建为单独的实体
- 提取所有关系：人物与组织、人物与地点、人物与时间、组织与地点等

**示例**（这只是格式示例，请根据实际文本完整提取）：
文本：陈海，男性，33岁，出生于月球基地（隶属ICMA管辖），中校军衔。毕业于科技与教育部（SED）直属深空科技大学，加入深空防卫军（DSOF）。

输出：
{
  "entities": [
    {"type": "Person", "name": "陈海", "attributes": {"gender": "男性", "age": "33岁", "rank": "中校"}},
    {"type": "Location", "name": "月球基地"},
    {"type": "Organization", "name": "ICMA"},
    {"type": "Organization", "name": "科技与教育部"},
    {"type": "Organization", "name": "SED"},
    {"type": "Organization", "name": "深空科技大学"},
    {"type": "Organization", "name": "深空防卫军"},
    {"type": "Organization", "name": "DSOF"}
  ],
  "relations": [
    {"from": "陈海", "to": "月球基地", "relation": "出生于"},
    {"from": "月球基地", "to": "ICMA", "relation": "隶属于"},
    {"from": "陈海", "to": "深空科技大学", "relation": "毕业于"},
    {"from": "深空科技大学", "to": "SED", "relation": "隶属于"},
    {"from": "陈海", "to": "深空防卫军", "relation": "加入"}
  ]
}

现在请从以下文本中**完整提取所有实体和关系**：
"#;

pub const KNOWLEDGE_FUSION_PROMPT: &str = r#"你是一个知识图谱管理专家。我会提供历史记忆和新记忆，你需要进行深度知识融合和推理。

**任务要求**：
1. **完整提取所有实体**：从新记忆中提取所有Person、Organization、Location、Time、Event类型的实体
2. **特别注意括号内容**：如"深空防卫军(DSOF)"中的DSOF也是Organization，要一起提取
3. **提取所有缩写组织**：如ICMA、HGA、SED、DSOF、DSTL、ISRI等都是独立的Organization实体
4. **识别实体合并**：判断不同名称是否指向同一实体（如"李明"和"我二哥"是同一人）
5. **推导隐含关系**：从已有关系推导新关系
6. **保留所有关系**：包括人物→组织、人物→地点、组织→地点、人物→时间、组织→组织等

**实体类型**：
- Person：人物姓名（attributes存储：identity身份、age年龄、rank军衔、gender性别等）
- Organization：组织机构名（包括全称和缩写，如ICMA、深空防卫军、科技与教育部等）
- Location：地点名（如月球基地、太空站、飞船、星球等）
- Time：时间信息（如地球历2067.6.16、UCT纪元7年、年龄等）
- Event：事件名（如战役、远征计划、测试项目等）

**关系类型示例**：
- 人物→组织：担任、工作于、加入、服役于、毕业于、指挥
- 人物→人物：同事、上下级、队友、战友
- 人物→地点：出生于、驻扎于、负责、来自
- 组织→地点：位于、管辖、隶属于
- 组织→组织：隶属于、直属于、管辖
- 人物→事件：参与、指挥、经历
- 人物→时间：出生于
- 实体→实体：隶属于、管辖

**重要规则**：
- **括号中的缩写必须提取**：如"深空防卫军(DSOF)"要提取两个Organization："深空防卫军"和"DSOF"，并建立"DSOF"作为"深空防卫军"的别名
- **所有组织都要提取**：无论是全称还是缩写，都是独立的Organization实体
- 如果实体A通过"是"/"又称"/"即"等关系指向实体B，则A和B是同一实体，应创建别名关系
- 从关系链推导隐含关系（如A是B, B工作于C => A工作于C）
- 属性信息放在attributes中，不要创建成单独的实体
- **不要遗漏任何组织、地点、时间实体**

**输出格式**：只输出JSON，示例：
{
  "entities": [
    {"type": "Person", "name": "陈海", "attributes": {"identity": "副舰长", "rank": "中校", "age": "33岁", "gender": "男性"}},
    {"type": "Organization", "name": "ICMA"},
    {"type": "Organization", "name": "星际殖民与移民总局"},
    {"type": "Organization", "name": "HGA"},
    {"type": "Organization", "name": "卫生健康与基因管理局"},
    {"type": "Organization", "name": "SED"},
    {"type": "Organization", "name": "科技与教育部"},
    {"type": "Organization", "name": "深空科技大学"},
    {"type": "Organization", "name": "深空防卫军"},
    {"type": "Organization", "name": "DSOF"},
    {"type": "Organization", "name": "深空科技实验室"},
    {"type": "Organization", "name": "DSTL"},
    {"type": "Organization", "name": "星际战略研究院"},
    {"type": "Organization", "name": "ISRI"},
    {"type": "Location", "name": "月球基地"},
    {"type": "Location", "name": "赤霄舰"},
    {"type": "Time", "name": "地球历2067.6.16"},
    {"type": "Time", "name": "UCT纪元7年6月16日"},
    {"type": "Event", "name": "赤霄舰远征计划"}
  ],
  "aliases": [
    {"primary": "星际殖民与移民总局", "alias": "ICMA"},
    {"primary": "卫生健康与基因管理局", "alias": "HGA"},
    {"primary": "科技与教育部", "alias": "SED"},
    {"primary": "深空防卫军", "alias": "DSOF"},
    {"primary": "深空科技实验室", "alias": "DSTL"},
    {"primary": "星际战略研究院", "alias": "ISRI"}
  ],
  "relations": [
    {"from": "陈海", "to": "月球基地", "relation": "出生于"},
    {"from": "月球基地", "to": "ICMA", "relation": "隶属于"},
    {"from": "陈海", "to": "深空科技大学", "relation": "毕业于"},
    {"from": "深空科技大学", "to": "SED", "relation": "直属于"},
    {"from": "陈海", "to": "深空防卫军", "relation": "加入"},
    {"from": "陈海", "to": "赤霄舰远征计划", "relation": "参与"}
  ]
}

---
历史记忆：
"#;

const KNOWLEDGE_FUSION_PROMPT_NEW: &str = "\n\n新记忆：\n";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    #[serde(rename = "type")]
    pub entity_type: String,
    pub name: String,
    #[serde(default)]
    pub attributes: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedRelation {
    pub from: String,
    pub to: String,
    pub relation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedData {
    pub entities: Vec<ExtractedEntity>,
    pub relations: Vec<ExtractedRelation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityAlias {
    pub primary: String,
    pub alias: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusedKnowledge {
    pub entities: Vec<ExtractedEntity>,
    #[serde(default)]
    pub aliases: Vec<EntityAlias>,
    pub relations: Vec<ExtractedRelation>,
}

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

pub const EXTRACT_ENTITY_PROMPT: &str = "从以下问题中提取要查询的人物或实体的名称，只输出名称，不要其他内容：\n问题：";
pub const ANSWER_PROMPT_PREFIX: &str = r#"你是一个记忆助手，只能根据以下已记录的记忆回答问题。

重要规则：
1. 只使用下方提供的记忆内容回答
2. 如果记忆中没有相关信息，明确说"记忆中没有相关信息"
3. 不要编造或推测记忆之外的内容
4. 回答要简洁准确

记忆：
"#;
pub const ANSWER_PROMPT_SUFFIX: &str = "\n\n问题：";

/// 检测 Ollama 是否已在运行（快速 GET 请求）
fn ollama_ping(base_url: &str) -> bool {
    let url = format!("{}/api/tags", base_url.trim_end_matches('/'));
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(3))
        .build();
    let client = match client {
        Ok(c) => c,
        Err(_) => return false,
    };
    client.get(&url).send().map_or(false, |r| r.status().is_success())
}

/// 公开检测接口：返回详细的状态信息
pub fn check_ollama_status(base_url: &str) -> (bool, String) {
    let url = format!("{}/api/tags", base_url.trim_end_matches('/'));
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(3))
        .build();
    let client = match client {
        Ok(c) => c,
        Err(e) => return (false, format!("创建 HTTP 客户端失败: {}", e)),
    };
    match client.get(&url).send() {
        Ok(resp) => {
            if resp.status().is_success() {
                (true, "Ollama 正在运行".to_string())
            } else {
                (false, format!("Ollama 返回状态码: {}", resp.status()))
            }
        }
        Err(e) => (false, format!("连接 Ollama 失败: {}", e)),
    }
}

/// 检查模型是否已下载
pub fn check_model_exists(base_url: &str, model: &str) -> bool {
    let url = format!("{}/api/tags", base_url.trim_end_matches('/'));
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(3))
        .build();
    let client = match client {
        Ok(c) => c,
        Err(_) => return false,
    };
    match client.get(&url).send() {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(json) = resp.json::<serde_json::Value>() {
                if let Some(models) = json.get("models").and_then(|v| v.as_array()) {
                    return models.iter().any(|m| {
                        m.get("name")
                            .and_then(|n| n.as_str())
                            .map_or(false, |n| n.starts_with(model))
                    });
                }
            }
            false
        }
        _ => false,
    }
}

/// 拉取模型（通过 Ollama API 的 /api/pull 端点）
pub fn pull_model(base_url: &str, model: &str) -> Result<String, String> {
    let url = format!("{}/api/pull", base_url.trim_end_matches('/'));
    let body = serde_json::json!({
        "name": model,
        "stream": false
    });
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(600)) // 10 分钟，模型下载可能较慢
        .build()
        .map_err(|e| e.to_string())?;
    let res = client
        .post(&url)
        .json(&body)
        .send()
        .map_err(|e| format!("拉取模型请求失败: {}", e))?;
    if !res.status().is_success() {
        let status = res.status();
        let err_body = res.text().unwrap_or_default();
        return Err(format!("拉取模型失败 {}: {}", status, err_body));
    }
    Ok(format!("模型 {} 下载完成", model))
}

/// 确保模型可用：若未下载则自动拉取
pub fn ensure_model_available(base_url: &str, model: &str) -> Result<(), String> {
    if check_model_exists(base_url, model) {
        return Ok(());
    }
    pull_model(base_url, model)?;
    Ok(())
}

/// 尝试在本地启动 ollama serve（仅当 base_url 为 localhost 时）
fn try_start_ollama_serve() {
    // macOS: 优先尝试启动 Ollama.app（会自动运行 serve）
    #[cfg(target_os = "macos")]
    {
        let app_path = "/Applications/Ollama.app";
        if std::path::Path::new(app_path).exists() {
            let _ = std::process::Command::new("open")
                .arg("-a")
                .arg(app_path)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
            return;
        }
    }
    
    // 其他情况：尝试命令行 ollama serve
    let cmd = std::env::consts::OS;
    let (bin, args): (&str, &[&str]) = if cmd == "windows" {
        ("ollama.exe", &["serve"][..])
    } else {
        ("ollama", &["serve"][..])
    };
    let _ = std::process::Command::new(bin)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

/// 确保 Ollama 可用：若未运行则尝试启动本机 ollama serve（仅针对 localhost）
pub fn ensure_ollama_running(base_url: &str) -> Result<(), String> {
    let base = base_url.trim_end_matches('/');
    if ollama_ping(base_url) {
        return Ok(());
    }
    // 仅对 localhost 尝试自动启动
    if base.contains("127.0.0.1") || base.contains("localhost") {
        try_start_ollama_serve();
        for i in 0..15 {
            if i > 0 {
                std::thread::sleep(Duration::from_millis(800));
            }
            if ollama_ping(base_url) {
                return Ok(());
            }
        }
    }
    Err("Ollama 未响应。请手动启动 Ollama.app 或在终端执行 ollama serve，并确保已执行：ollama pull qwen2.5:1.5b 与 ollama pull qwen2.5:7b".to_string())
}

/// 将 Ollama 连接/HTTP 错误转为带提示的友好说明
fn ollama_error_hint(err: String) -> String {
    let lower = err.to_lowercase();
    if lower.contains("502") || lower.contains("bad gateway")
        || lower.contains("connection refused")
        || lower.contains("connection reset")
    {
        return format!(
            "无法连接 Ollama。\n请确认：\n1. 已安装 Ollama 并正在运行；\n2. 终端执行 ollama serve 或从应用启动；\n3. 已拉取模型：ollama pull qwen2.5:1.5b（实体拆分）、ollama pull qwen2.5:7b（问答）\n\n原始错误：{}",
            err
        );
    }
    err
}

pub fn call_ollama_simple(base_url: &str, model: &str, prompt: &str) -> Result<String, String> {
    let url = format!("{}/api/generate", base_url.trim_end_matches('/'));
    let body = serde_json::json!({
        "model": model,
        "prompt": prompt,
        "stream": false,
        "options": { "temperature": 0.3, "num_predict": 512 }
    });
    let client = reqwest::blocking::Client::new();
    let res = client
        .post(&url)
        .json(&body)
        .send()
        .map_err(|e| ollama_error_hint(e.to_string()))?;
    if !res.status().is_success() {
        let status = res.status();
        let err_body = res.text().unwrap_or_default();
        return Err(ollama_error_hint(format!("{} {}", status, err_body)));
    }
    let json: serde_json::Value = res.json().map_err(|e| ollama_error_hint(e.to_string()))?;
    let response_text = json
        .get("response")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    Ok(response_text)
}

pub fn call_ollama_extract_blocking(base_url: &str, model: &str, text: &str) -> Result<ExtractedData, String> {
    let prompt = format!("{}{}", ENTITY_EXTRACT_PROMPT, text);
    let url = format!("{}/api/generate", base_url.trim_end_matches('/'));
    let body = serde_json::json!({
        "model": model,
        "prompt": prompt,
        "stream": false,
        "options": { "temperature": 0.2, "num_predict": 4096 }
    });
    let client = reqwest::blocking::Client::new();
    let res = client
        .post(&url)
        .json(&body)
        .send()
        .map_err(|e| ollama_error_hint(e.to_string()))?;
    if !res.status().is_success() {
        let status = res.status();
        let err_body = res.text().unwrap_or_default();
        return Err(ollama_error_hint(format!("Ollama error {}: {}", status, err_body)));
    }
    let json: serde_json::Value = res.json().map_err(|e| ollama_error_hint(e.to_string()))?;
    let response_text = json
        .get("response")
        .and_then(|v| v.as_str())
        .ok_or("missing response field")?;
    let json_str = extract_json_from_response(response_text).ok_or("could not extract JSON from response")?;
    let data: ExtractedData = serde_json::from_str(json_str).map_err(|e| format!("parse JSON: {}", e))?;
    Ok(data)
}

/// 知识融合推理：结合历史记忆和新记忆，进行实体合并和关系推导
pub fn call_ollama_knowledge_fusion(
    base_url: &str,
    model: &str,
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
    
    let prompt = format!(
        "{}{}{}{}",
        KNOWLEDGE_FUSION_PROMPT,
        historical_text,
        KNOWLEDGE_FUSION_PROMPT_NEW,
        new_memory
    );
    
    let url = format!("{}/api/generate", base_url.trim_end_matches('/'));
    let body = serde_json::json!({
        "model": model,
        "prompt": prompt,
        "stream": false,
        "options": { "temperature": 0.2, "num_predict": 6144 }
    });
    
    let client = reqwest::blocking::Client::new();
    let res = client
        .post(&url)
        .json(&body)
        .send()
        .map_err(|e| ollama_error_hint(e.to_string()))?;
        
    if !res.status().is_success() {
        let status = res.status();
        let err_body = res.text().unwrap_or_default();
        return Err(ollama_error_hint(format!("Ollama error {}: {}", status, err_body)));
    }
    
    let json: serde_json::Value = res.json().map_err(|e| ollama_error_hint(e.to_string()))?;
    let response_text = json
        .get("response")
        .and_then(|v| v.as_str())
        .ok_or("missing response field")?;
        
    let json_str = extract_json_from_response(response_text)
        .ok_or("could not extract JSON from response")?;
        
    let data: FusedKnowledge = serde_json::from_str(json_str)
        .map_err(|e| format!("parse JSON: {}", e))?;
        
    Ok(data)
}
