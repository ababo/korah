use crate::{
    llm::{
        open_ai::{create_request_tools, RequestTool, Role},
        BoxLlm, Error, LlmClient, ToolCall,
    },
    tool::ToolMeta,
};
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
}

impl OllamaClient {
    /// Creates a boxed Ollama instance.
    pub fn new_boxed(config: OllamaConfig) -> BoxLlm {
        Box::new(Self { config })
    }
}

impl LlmClient for OllamaClient {
    fn derive_tool_call(
        &self,
        tools: Vec<ToolMeta>,
        query: String,
    ) -> Result<Option<ToolCall>, Error> {
        let messages = vec![Message {
            role: Role::User,
            content: query,
            tool_calls: vec![],
        }];
        let request = ChatRequestPayload {
            model: self.config.model.clone(),
            messages,
            stream: false,
            tools: create_request_tools(tools),
        };

        let mut url = self.config.base_url.clone();
        url.set_path(&format!("{}api/chat", url.path()));

        let response: ChatResponsePayload =
            ureq::post(url.as_str()).send_json(request)?.into_json()?;

        Ok(create_tool_call(response))
    }
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

#[derive(Deserialize, Serialize)]
struct Message {
    role: Role,
    content: String,
    #[serde(default)]
    tool_calls: Vec<ResponseToolCall>,
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
