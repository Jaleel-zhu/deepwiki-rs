use anyhow::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use std::path::Path;

use crate::generator::agent_executor::{AgentExecuteParams, extract};
use crate::generator::context::GeneratorContext;
use crate::types::code::{CodePurpose, CodePurposeMapper};

fn deserialize_code_purpose_from_any<'de, D>(deserializer: D) -> Result<CodePurpose, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    let raw = match value {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(s) => s,
        serde_json::Value::Bool(v) => v.to_string(),
        serde_json::Value::Number(v) => v.to_string(),
        serde_json::Value::Array(v) => serde_json::to_string(&v).unwrap_or_default(),
        serde_json::Value::Object(v) => serde_json::to_string(&v).unwrap_or_default(),
    };
    Ok(CodePurposeMapper::map_from_raw(&raw))
}

fn deserialize_f64_lenient<'de, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    let result = match value {
        serde_json::Value::Number(n) => n.as_f64().unwrap_or(0.0),
        serde_json::Value::String(s) => s.parse::<f64>().unwrap_or(0.0),
        serde_json::Value::Bool(v) => {
            if v {
                1.0
            } else {
                0.0
            }
        }
        _ => 0.0,
    };
    Ok(result)
}

fn deserialize_string_lenient<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    let result = match value {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(s) => s,
        serde_json::Value::Bool(v) => v.to_string(),
        serde_json::Value::Number(v) => v.to_string(),
        serde_json::Value::Array(v) => serde_json::to_string(&v).unwrap_or_default(),
        serde_json::Value::Object(v) => serde_json::to_string(&v).unwrap_or_default(),
    };
    Ok(result)
}

/// AI component type analysis result
#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(default)]
pub struct AICodePurposeAnalysis {
    // Inferred code functionality classification
    #[serde(default, deserialize_with = "deserialize_code_purpose_from_any")]
    pub code_purpose: CodePurpose,
    // Confidence of the inference result (min 0.0, max 1.0), confidence is high when > 0.7.
    #[serde(default, deserialize_with = "deserialize_f64_lenient")]
    pub confidence: f64,
    #[serde(default, deserialize_with = "deserialize_string_lenient")]
    pub reasoning: String,
}

/// Component type enhancer, combining rules and AI analysis
pub struct CodePurposeEnhancer;

impl CodePurposeEnhancer {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn execute(
        &self,
        context: &GeneratorContext,
        file_path: &Path,
        file_name: &str,
        file_content: &str,
    ) -> Result<CodePurpose> {
        // First use rule mapping
        let rule_based_type =
            CodePurposeMapper::map_by_path_and_name(&file_path.to_string_lossy(), file_name);

        // If rule mapping gets explicit type with high confidence, return directly
        if rule_based_type != CodePurpose::Other {
            return Ok(rule_based_type);
        }

        // If there's AI analyzer and file content, use AI enhanced analysis
        let prompt_sys = "You are a professional code architecture analyst specializing in analyzing component types of code files.".to_string();
        let prompt_user =
            self.build_code_purpose_analysis_prompt(file_path, file_content, file_name);

        let analyze_result = extract::<AICodePurposeAnalysis>(
            context,
            AgentExecuteParams {
                prompt_sys,
                prompt_user,
                cache_scope: "ai_code_purpose".to_string(),
                log_tag: file_name.to_string(),
            },
        )
        .await;

        return match analyze_result {
            Ok(ai_analysis) => {
                // If AI analysis confidence is high, use AI result
                if ai_analysis.confidence > 0.7 {
                    return Ok(ai_analysis.code_purpose);
                }
                // Otherwise combine rule and AI results
                if rule_based_type != CodePurpose::Other {
                    Ok(rule_based_type)
                } else {
                    Ok(ai_analysis.code_purpose)
                }
            }
            Err(_) => {
                // AI analysis failed, use rule result
                Ok(rule_based_type)
            }
        };
    }

    /// Build component type analysis prompt
    fn build_code_purpose_analysis_prompt(
        &self,
        file_path: &Path,
        file_content: &str,
        file_name: &str,
    ) -> String {
        // Safely truncate first 1000 characters of file content for analysis
        let content_preview = if file_content.chars().count() > 1000 {
            let truncated: String = file_content.chars().take(1000).collect();
            format!("{}...", truncated)
        } else {
            file_content.to_string()
        };

        format!(
            include_str!("prompts/code_purpose_analyze_user.tpl"),
            file_path.display(),
            file_name,
            content_preview
        )
    }
}

#[cfg(test)]
mod tests {
    use super::AICodePurposeAnalysis;
    use crate::types::code::CodePurpose;

    #[test]
    fn test_ai_code_purpose_analysis_deserialize_unknown_variant_text() {
        let payload = serde_json::json!({
            "code_purpose": "Migration configuration script (Alembic env file)",
            "confidence": "0.91",
            "reasoning": {"summary":"matched migration config"}
        });

        let parsed: AICodePurposeAnalysis = serde_json::from_value(payload)
            .expect("AICodePurposeAnalysis should deserialize loose purpose variant");

        assert_eq!(parsed.code_purpose, CodePurpose::Config);
        assert_eq!(parsed.confidence, 0.91);
    }

    #[test]
    fn test_ai_code_purpose_analysis_deserialize_short_service_api_text() {
        let payload = serde_json::json!({
            "code_purpose": "Service API for external calls",
            "confidence": 0.8,
            "reasoning": "api classification"
        });

        let parsed: AICodePurposeAnalysis = serde_json::from_value(payload)
            .expect("AICodePurposeAnalysis should deserialize shortened API variant");

        assert_eq!(parsed.code_purpose, CodePurpose::Api);
    }
}
