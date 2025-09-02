//! LLM prompts and templates

use crate::models::Message;
use std::collections::HashMap;

/// Template for LLM prompts
#[derive(Debug, Clone)]
pub struct PromptTemplate {
    /// Template name
    pub name: String,
    /// Template content with placeholders
    pub template: String,
    /// Default values for placeholders
    pub defaults: HashMap<String, String>,
}

impl PromptTemplate {
    /// Create a new prompt template
    #[must_use]
    pub fn new(name: impl Into<String>, template: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            template: template.into(),
            defaults: HashMap::new(),
        }
    }

    /// Add a default value for a placeholder
    #[must_use]
    pub fn with_default(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.defaults.insert(key.into(), value.into());
        self
    }

    /// Render the template with provided values
    #[must_use]
    pub fn render(&self, values: &HashMap<String, String>) -> String {
        let mut result = self.template.clone();

        // First apply provided values
        for (key, value) in values {
            let placeholder = format!("{{{key}}}");
            result = result.replace(&placeholder, value);
        }

        // Then apply defaults for any remaining placeholders
        for (key, value) in &self.defaults {
            let placeholder = format!("{{{key}}}");
            result = result.replace(&placeholder, value);
        }

        result
    }
}

/// Prompt builder for constructing LLM conversations
#[derive(Debug, Clone)]
pub struct PromptBuilder {
    /// System message (optional)
    system_message: Option<String>,
    /// User messages
    messages: Vec<Message>,
    /// Available templates
    templates: HashMap<String, PromptTemplate>,
}

impl Default for PromptBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl PromptBuilder {
    /// Create a new prompt builder
    #[must_use]
    pub fn new() -> Self {
        let mut builder = Self {
            system_message: None,
            messages: Vec::new(),
            templates: HashMap::new(),
        };

        // Register default templates
        builder.register_default_templates();
        builder
    }

    /// Register built-in templates
    fn register_default_templates(&mut self) {
        // Summarization template
        let summary_template = PromptTemplate::new(
            "summarize",
            "Please analyze this text and provide a concise summary:\n\n{text}",
        );

        // Code analysis template
        let code_template = PromptTemplate::new(
            "analyze_code",
            "Please analyze the following code and provide insights:\n\n```language\n{code}\n```\n\n{analysis_type}"
        )
        .with_default("analysis_type", "Provide a summary of what this code does and any notable patterns or issues.");

        // Documentation template
        let docs_template = PromptTemplate::new(
            "document_analysis",
            "Analyze this documentation and extract key information:\n\n{content}\n\n{focus}",
        )
        .with_default(
            "focus",
            "Focus on API endpoints, configuration options, and usage examples.",
        );

        self.templates
            .insert("summarize".to_string(), summary_template);
        self.templates
            .insert("analyze_code".to_string(), code_template);
        self.templates
            .insert("document_analysis".to_string(), docs_template);
    }

    /// Add a custom template
    pub fn add_template(&mut self, template: PromptTemplate) {
        self.templates.insert(template.name.clone(), template);
    }

    /// Set system message
    #[must_use]
    pub fn with_system(mut self, message: impl Into<String>) -> Self {
        self.system_message = Some(message.into());
        self
    }

    /// Add a user message
    #[must_use]
    pub fn with_message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }

    /// Add a user message from text
    #[must_use]
    pub fn with_user_message(mut self, content: impl Into<String>) -> Self {
        self.messages.push(Message::user(content));
        self
    }

    /// Use a template to create a message
    #[must_use]
    pub fn with_template(mut self, template_name: &str, values: &HashMap<String, String>) -> Self {
        if let Some(template) = self.templates.get(template_name) {
            let content = template.render(values);
            self.messages.push(Message::user(content));
        }
        self
    }

    /// Build the final message list for the LLM
    #[must_use]
    pub fn build(&self) -> Vec<Message> {
        let mut messages = Vec::new();

        // Add system message first if present
        if let Some(system) = &self.system_message {
            messages.push(Message::system(system.clone()));
        }

        // Add all other messages
        messages.extend(self.messages.clone());

        messages
    }

    /// Build for Claude Code (simplified format)
    #[must_use]
    pub fn build_for_claude_code(&self) -> String {
        let messages = self.build();
        let mut prompt = String::new();

        for message in messages {
            match message.role.as_str() {
                "system" => {
                    prompt.push_str("System: ");
                    prompt.push_str(&message.content);
                    prompt.push('\n');
                }
                "user" => {
                    prompt.push_str("User: ");
                    prompt.push_str(&message.content);
                    prompt.push('\n');
                }
                "assistant" => {
                    prompt.push_str("Assistant: ");
                    prompt.push_str(&message.content);
                    prompt.push('\n');
                }
                _ => {}
            }
        }

        prompt.push_str("Assistant: ");
        prompt
    }

    /// Build for `OpenAI` API format
    #[must_use]
    pub fn build_for_openai(&self) -> Vec<Message> {
        self.build()
    }
}

/// Pre-configured prompt factory
pub struct PromptFactory;

impl PromptFactory {
    /// Create a summarization prompt
    #[must_use]
    pub fn summarize_text(text: &str) -> PromptBuilder {
        let mut values = HashMap::new();
        values.insert("text".to_string(), text.to_string());

        PromptBuilder::new().with_template("summarize", &values)
    }

    /// Create a code analysis prompt
    #[must_use]
    pub fn analyze_code(code: &str, language: &str, analysis_type: Option<&str>) -> PromptBuilder {
        let mut values = HashMap::new();
        values.insert("code".to_string(), code.to_string());
        values.insert("language".to_string(), language.to_string());

        if let Some(analysis) = analysis_type {
            values.insert("analysis_type".to_string(), analysis.to_string());
        }

        PromptBuilder::new().with_template("analyze_code", &values)
    }

    /// Create a documentation analysis prompt
    #[must_use]
    pub fn analyze_documentation(content: &str, focus: Option<&str>) -> PromptBuilder {
        let mut values = HashMap::new();
        values.insert("content".to_string(), content.to_string());

        if let Some(focus_area) = focus {
            values.insert("focus".to_string(), focus_area.to_string());
        }

        PromptBuilder::new().with_template("document_analysis", &values)
    }
}
