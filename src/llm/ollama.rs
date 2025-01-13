use crate::{
    llm::{BoxLlm, Error, LlmClient, ToolCall},
    tool::ToolMeta,
};
use log::{debug, log_enabled};
use schemars::schema::SingleOrVec;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use url::Url;

/// An Ollama LLM API configuration.
#[derive(Clone, Debug, Deserialize)]
pub struct OllamaConfig {
    pub base_url: Url,
    pub model: String,
}

/// An Ollama API client.
pub struct OllamaClient {
    config: OllamaConfig,
    tools: Vec<RequestTool>,
}

impl OllamaClient {
    /// Creates a boxed Ollama instance.
    pub fn new_boxed(config: OllamaConfig, tools: Vec<ToolMeta>) -> BoxLlm {
        let tools = create_request_tools(tools);
        if log_enabled!(log::Level::Debug) {
            debug!("ollama tools {}", serde_json::to_string(&tools).unwrap());
        }
        Box::new(Self { config, tools })
    }
}

impl LlmClient for OllamaClient {
    fn derive_tool_call(&self, query: &str) -> Result<Option<ToolCall>, Error> {
        let messages = vec![Message {
            role: Role::User,
            content: query.to_owned(),
            tool_calls: vec![],
        }];
        let request = ChatRequestPayload {
            model: self.config.model.clone(),
            messages,
            stream: false,
            tools: self.tools.clone(),
        };

        let mut url = self.config.base_url.clone();
        url.set_path(&format!("{}api/chat", url.path()));

        let response: ChatResponsePayload =
            ureq::post(url.as_str()).send_json(request)?.into_json()?;

        Ok(create_tool_call(response))
    }
}

fn create_request_tools(tools: Vec<ToolMeta>) -> Vec<RequestTool> {
    tools
        .into_iter()
        .map(|t| {
            let mut params = t.params_schema.schema.object.unwrap();

            // Enforce single instance types since Ollama doesn't support arrays.
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

fn create_tool_call(response: ChatResponsePayload) -> Option<ToolCall> {
    let mut calls = response.message.tool_calls;
    if calls.len() == 1 {
        let call = calls.remove(0);
        Some(ToolCall {
            tool: call.function.name,
            params: call.function.arguments,
        })
    } else {
        None
    }
}

#[derive(Serialize)]
struct ChatRequestPayload {
    model: String,
    messages: Vec<Message>,
    stream: bool,
    tools: Vec<RequestTool>,
}

#[allow(dead_code)]
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum Role {
    Assistant,
    System,
    Tool,
    User,
}

#[derive(Deserialize, Serialize)]
struct Message {
    role: Role,
    content: String,
    #[serde(default)]
    tool_calls: Vec<ResponseToolCall>,
}

#[derive(Clone, Serialize)]
struct RequestTool {
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
    message: Message,
}

#[derive(Deserialize, Serialize)]
struct ResponseToolCall {
    function: ResponseToolCallFunction,
}

#[derive(Deserialize, Serialize)]
struct ResponseToolCallFunction {
    name: String,
    arguments: Box<RawValue>,
}
