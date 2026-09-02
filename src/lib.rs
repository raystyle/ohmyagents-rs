pub mod agents;
pub mod api;
pub mod caps;
pub mod catalog;
pub mod deploy;
pub mod doctor;
pub mod fmtio;
pub mod hook;
pub mod install;
pub mod login;
pub mod orch;
pub mod pathutil;
pub mod providers;
pub mod repl;
pub mod rmux;
pub mod rmuxpoc;
pub mod secretguard;
pub mod secrets;
pub mod statusline;
pub mod task;
pub mod trace;
pub mod update;
pub mod webassets;
pub mod yolo;

// P0011 传输适配层：HTTP 进 server feature，MCP 进 mcp feature；api 层无新依赖不设门。

/// 测试共享 env 互斥：动 OMA_HOME / SOPS_AGE_KEY_FILE 的测试跨模块也要
/// 互斥（各自局部锁挡不住并发互踩）。
#[cfg(test)]
pub(crate) mod testenv {
    pub static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
}
#[cfg(feature = "mcp")]
pub mod mcp;
pub mod servectl;
#[cfg(feature = "server")]
pub mod server;
