use crate::{
    api::tool::ToolMetadata,
    llm::{BoxLlm, Error, Llm, ToolCall as LlmToolCall},
};
use futures::future::BoxFuture;
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

/// An Ollama LLM client.
pub struct Ollama {
    base_url: Url,
    client: Client,
}

#[derive(Serialize)]
struct PullRequestPayload {
    model: String,
    insecure: bool,
    stream: bool,
}

impl Ollama {
    /// Creates an Ollama instance in a form of BoxLlm.
    pub fn new_boxed(base_url: Url) -> Result<BoxLlm, Error> {
        if base_url.cannot_be_a_base() {
            return Err(Error::UnsupportedUrl);
        }
        Ok(Box::new(Self {
            base_url,
            client: Client::new(),
        }))
    }
}

impl Llm for Ollama {
    fn prepare_model(&self, model: &str) -> BoxFuture<Result<(), Error>> {
        let request = PullRequestPayload {
            model: model.to_owned(),
            insecure: false,
            stream: false,
        };

        let mut url = self.base_url.clone();
        url.set_path(&format!("{}api/pull", url.path()));

        let client = self.client.clone();

        Box::pin(async move {
            client
                .post(url)
                .json(&request)
                .send()
                .await?
                .error_for_status()?;

            Ok(())
        })
    }

    fn derive_tool_call(
        &self,
        model: String,
        tools: Vec<ToolMetadata>,
        query: String,
    ) -> BoxFuture<Result<Option<LlmToolCall>, Error>> {
        let messages = vec![Message {
            role: Role::User,
            content: query,
            tool_calls: vec![],
        }];
        let request = ChatRequestPayload {
            model,
            messages,
            stream: false,
            tools: compose_tools(tools),
        };

        let mut url = self.base_url.clone();
        url.set_path(&format!("{}api/chat", url.path()));

        let client = self.client.clone();

        Box::pin(async move {
            let response: ChatResponsePayload = client
                .post(url)
                .json(&request)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?;
            Ok(compose_call(response))
        })
    }
}

fn compose_tools(tools: Vec<ToolMetadata>) -> Vec<Tool> {
    tools
        .into_iter()
        .map(|t| {
            let object = t.params_schema.object.unwrap();
            let required: Vec<String> = object.required.into_iter().collect();
            let properties = serde_json::to_string(&object.properties).unwrap();
            let properties = RawValue::from_string(properties).unwrap();
            let function = ToolFunction {
                name: t.name,
                description: t.description,
                parameters: Parameters::new(required, properties),
            };
            Tool::new(function)
        })
        .collect()
}

fn compose_call(response: ChatResponsePayload) -> Option<LlmToolCall> {
    let mut calls = response.message.tool_calls;
    if calls.len() == 1 {
        let call = calls.remove(0);
        Some(LlmToolCall {
            name: call.function.name,
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
    tools: Vec<Tool>,
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
    tool_calls: Vec<ToolCall>,
}

#[derive(Serialize)]
struct Tool {
    r#type: &'static str,
    function: ToolFunction,
}

impl Tool {
    fn new(function: ToolFunction) -> Self {
        Self {
            r#type: "function",
            function,
        }
    }
}

#[derive(Serialize)]
struct ToolFunction {
    name: String,
    description: Option<String>,
    parameters: Parameters,
}

#[derive(Serialize)]
struct Parameters {
    r#type: &'static str,
    required: Vec<String>,
    properties: Box<RawValue>,
}

impl Parameters {
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
struct ToolCall {
    function: ToolCallFunction,
}

#[derive(Deserialize, Serialize)]
struct ToolCallFunction {
    name: String,
    arguments: Box<RawValue>,
}
