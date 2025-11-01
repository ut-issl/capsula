mod config;
mod error;
use crate::config::SlackNotifyHookConfig;
use crate::error::SlackNotifyError;
use capsula_core::captured::Captured;
use capsula_core::error::CapsulaResult;
use capsula_core::hook::{Hook, HookFactory, PostRun, PreRun, RuntimeParams};
use capsula_core::run::PreparedRun;
use serde::Serialize;
use serde_json::json;

pub const KEY: &str = "notify-slack";

#[derive(Debug)]
pub struct SlackNotifyHook {
    pub config: SlackNotifyHookConfig,
}

#[derive(Debug, Serialize)]
pub struct SlackNotifyCaptured {
    pub message: String,
    pub response: Option<String>,
}

impl Captured for SlackNotifyCaptured {
    fn serialize_json(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }
}

impl Hook<PreRun> for SlackNotifyHook {
    type Config = SlackNotifyHookConfig;
    type Output = SlackNotifyCaptured;

    fn id(&self) -> String {
        KEY.to_string()
    }

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn run(
        &self,
        metadata: &PreparedRun,
        _params: &RuntimeParams<PreRun>,
    ) -> CapsulaResult<Self::Output> {
        let client = reqwest::blocking::Client::new();
        let payload = json!({
            "channel": self.config.channel,
            "text": format!("Run `{}` (ID: `{}`) is starting.", metadata.name, metadata.id),
        });

        let res = client
            .post("https://slack.com/api/chat.postMessage")
            .bearer_auth(&self.config.token)
            .json(&payload)
            .send()
            .map_err(SlackNotifyError::from)?;

        if !res.status().is_success() {
            return Err(SlackNotifyError::SlackApi {
                message: format!("Failed to send Slack notification: {}", res.status()),
            }
            .into());
        }
        let res_json = res.text().ok();

        Ok(SlackNotifyCaptured {
            message: "Slack notification sent successfully".to_string(),
            response: res_json,
        })
    }
}

impl Hook<PostRun> for SlackNotifyHook {
    type Config = SlackNotifyHookConfig;
    type Output = SlackNotifyCaptured;

    fn id(&self) -> String {
        KEY.to_string()
    }

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn run(
        &self,
        metadata: &PreparedRun,
        _params: &RuntimeParams<PostRun>,
    ) -> CapsulaResult<Self::Output> {
        let client = reqwest::blocking::Client::new();
        let payload = json!({
            "channel": self.config.channel,
            "text": format!("Run `{}` (ID: `{}`) has completed.", metadata.name, metadata.id),
        });

        let res = client
            .post("https://slack.com/api/chat.postMessage")
            .bearer_auth(&self.config.token)
            .json(&payload)
            .send()
            .map_err(SlackNotifyError::from)?;

        if !res.status().is_success() {
            return Err(SlackNotifyError::SlackApi {
                message: format!("Failed to send Slack notification: {}", res.status()),
            }
            .into());
        }

        Ok(SlackNotifyCaptured {
            message: "Slack notification sent successfully".to_string(),
            response: res.text().ok(),
        })
    }
}

pub fn create_pre_run_hook_factory() -> Box<dyn HookFactory<PreRun>> {
    Box::new(config::SlackNotifyHookFactory)
}

pub fn create_post_run_hook_factory() -> Box<dyn HookFactory<PostRun>> {
    Box::new(config::SlackNotifyHookFactory)
}
