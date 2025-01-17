use crate::{
    llm::{BoxLlm, Error, LlmClient, ToolCall},
    tool::ToolMeta,
};
use schemars::schema::SingleOrVec;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use url::Url;

/// An OpenAI LLM API configuration.
#[derive(Clone, Debug, Deserialize)]
pub struct OpenAiConfig {
    pub base_url: Url,
    pub key: String,
    pub model: String,
}

/// An Ollama API client.
pub struct OpenAiClient {
    config: OpenAiConfig,
}

impl OpenAiClient {
    /// Creates a boxed Ollama instance.
    pub fn new_boxed(config: OpenAiConfig) -> BoxLlm {
        Box::new(Self { config })
    }
}

impl LlmClient for OpenAiClient {
    fn derive_tool_call(
        &self,
        tools: Vec<ToolMeta>,
        query: String,
    ) -> Result<Option<ToolCall>, Error> {
        let messages = vec![Message {
            role: Role::User,
            content: Some(query),
            tool_calls: vec![],
        }];
        let request = ChatRequestPayload {
            model: self.config.model.clone(),
            messages,
            stream: false,
            tools: create_request_tools(tools),
        };

        let mut url = self.config.base_url.clone();
        url.set_path(&format!("{}/chat/completions", url.path()));

        let key = shellexpand::env(&self.config.key)?;

        let response: ChatResponsePayload = ureq::post(url.as_str())
            .set("Authorization", &format!("Bearer {key}"))
            .send_json(request)?
            .into_json()?;

        create_tool_call(response)
    }
}

#[derive(Serialize)]
pub(in crate::llm) struct ChatRequestPayload {
    model: String,
    messages: Vec<Message>,
    stream: bool,
    tools: Vec<RequestTool>,
}

#[allow(dead_code)]
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub(in crate::llm) enum Role {
    Assistant,
    System,
    Tool,
    User,
}

#[derive(Deserialize, Serialize)]
pub(in crate::llm) struct Message {
    role: Role,
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<ResponseToolCall>,
}

#[derive(Clone, Serialize)]
pub(in crate::llm) struct RequestTool {
    r#type: &'static str,
    function: RequestToolFunction,
}

impl RequestTool {
    fn new(function: RequestToolFunction) -> Self {
        Self {
            r#type: "function",
            function,
        }
    }
}

#[derive(Clone, Serialize)]
struct RequestToolFunction {
    name: String,
    description: Option<String>,
    parameters: RequestToolParameters,
}

#[derive(Clone, Serialize)]
struct RequestToolParameters {
    r#type: &'static str,
    required: Vec<String>,
    properties: Box<RawValue>,
}

impl RequestToolParameters {
    fn new(required: Vec<String>, properties: Box<RawValue>) -> Self {
        Self {
            r#type: "object",
            required,
            properties,
        }
    }
}

#[derive(Deserialize)]
struct ChatResponsePayload {
    choices: Vec<ResponseChoice>,
}

#[derive(Deserialize)]
struct ResponseChoice {
    message: Message,
}

#[derive(Deserialize, Serialize)]
struct ResponseToolCall {
    function: ResponseToolCallFunction,
}

#[derive(Deserialize, Serialize)]
struct ResponseToolCallFunction {
    name: String,
    arguments: String,
}

pub(in crate::llm) fn create_request_tools(tools: Vec<ToolMeta>) -> Vec<RequestTool> {
    tools
        .into_iter()
        .map(|t| {
            let mut params = t.params_schema.schema.object.unwrap();

            // Enforce single instance types since some compatible APIs don't support arrays.
            for (_, property) in params.properties.iter_mut() {
                let mut property_object = property.clone().into_object();
                property_object.instance_type = property_object.instance_type.map(|t| match t {
                    SingleOrVec::Vec(mut v) => SingleOrVec::Single(Box::new(v.remove(0))),
                    s => s,
                });
                *property = property_object.into();
            }

            let properties = serde_json::to_string(&params.properties).unwrap();
            let properties = RawValue::from_string(properties).unwrap();

            let required: Vec<String> = params.required.into_iter().collect();

            let function = RequestToolFunction {
                name: t.name,
                description: t.description,
                parameters: RequestToolParameters::new(required, properties),
            };
            RequestTool::new(function)
        })
        .collect()
}

fn create_tool_call(mut response: ChatResponsePayload) -> Result<Option<ToolCall>, Error> {
    if response.choices.is_empty() {
        return Ok(None);
    }
    let choice = response.choices.remove(0);

    let mut calls = choice.message.tool_calls;
    if calls.len() != 1 {
        return Ok(None);
    }
    let call = calls.remove(0);

    let tool = call.function.name;
    let params = serde_json::from_str(&call.function.arguments)?;
    Ok(Some(ToolCall { tool, params }))
}
