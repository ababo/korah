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
    #[serde(flatten)]
    pub options: OllamaOptions,
}

/// Ollama request options.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OllamaOptions {
    frequency_penalty: Option<f32>,
    low_vram: Option<bool>,
    main_gpu: Option<i32>,
    min_p: Option<f32>,
    mirostat_eta: Option<f32>,
    mirostat_tau: Option<f32>,
    mirostat: Option<i32>,
    num_batch: Option<i32>,
    num_ctx: Option<i32>,
    num_gpu: Option<i32>,
    num_keep: Option<i32>,
    num_predict: Option<i32>,
    num_thread: Option<i32>,
    numa: Option<bool>,
    penalize_newline: Option<bool>,
    presence_penalty: Option<f32>,
    repeat_last_n: Option<f32>,
    repeat_penalty: Option<f32>,
    seed: Option<i32>,
    stop: Option<Vec<String>>,
    temperature: Option<f32>,
    top_k: Option<i32>,
    top_p: Option<f32>,
    typical_p: Option<f32>,
    use_mlock: Option<bool>,
    use_mmap: Option<bool>,
    vocab_only: Option<bool>,
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
            options: self.config.options.clone(),
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
    options: OllamaOptions,
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
