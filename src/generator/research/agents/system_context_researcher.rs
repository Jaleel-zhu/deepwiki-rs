use crate::generator::{
    step_forward_agent::{
        AgentDataConfig, DataSource, FormatterConfig, LLMCallMode, PromptTemplate, StepForwardAgent,
    }
};
use crate::generator::research::memory::MemoryScope;
use crate::generator::research::types::{AgentType, SystemContextReport};

/// Project Objective Researcher - Responsible for analyzing the project's core objectives, functional value, and system boundaries
#[derive(Default)]
pub struct SystemContextResearcher;

impl StepForwardAgent for SystemContextResearcher {
    type Output = SystemContextReport;

    fn agent_type(&self) -> String {
        AgentType::SystemContextResearcher.to_string()
    }

    fn agent_type_enum(&self) -> Option<AgentType> {
        Some(AgentType::SystemContextResearcher)
    }

    fn memory_scope_key(&self) -> String {
        MemoryScope::STUDIES_RESEARCH.to_string()
    }

    fn data_config(&self) -> AgentDataConfig {
        AgentDataConfig {
            required_sources: vec![DataSource::PROJECT_STRUCTURE, DataSource::CODE_INSIGHTS],
            optional_sources: vec![
                DataSource::README_CONTENT,
                // Use architecture and ADR docs for system context analysis
                DataSource::knowledge_categories(vec!["architecture", "adr"]),
            ],
        }
    }

    fn prompt_template(&self) -> PromptTemplate {
        PromptTemplate {
            system_prompt: r#"You are a professional software architecture analyst, specializing in project objective and system boundary analysis.

Your task is to analyze and determine based on the provided project information:
1. The project's core objectives and business value
2. Project type and technical characteristics
3. Target user groups and usage scenarios
4. External system interactions
5. System boundary definition

You may have access to existing product description, requirements and architecture documentation from external sources.
If available, use this documentation to enhance your analysis with established business context and architectural decisions.
Validate code findings against documented architecture and identify any gaps or inconsistencies.

Please return the analysis results in structured JSON format."#
                .to_string(),

            opening_instruction: "Based on the following research materials, analyze the project's core objectives and system positioning:".to_string(),

            closing_instruction: r#"
## Analysis Requirements:
- Accurately identify project type and technical characteristics
- Clearly define target users and usage scenarios
- Clearly delineate system boundaries
- If external documentation is provided, validate code structure against it
- Identify any gaps between documented architecture and actual implementation
- Ensure analysis results conform to the C4 architecture model's system context level"#
                .to_string(),

            llm_call_mode: LLMCallMode::Extract,
            formatter_config: FormatterConfig::default(),
        }
    }
}
