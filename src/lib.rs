pub mod agents;
pub mod api;
pub mod caps;
pub mod catalog;
pub mod deploy;
pub mod doctor;
pub mod hook;
pub mod install;
pub mod orch;
pub mod pathutil;
pub mod providers;
pub mod repl;
pub mod rmux;
pub mod rmuxpoc;
pub mod statusline;
pub mod task;
pub mod trace;
pub mod update;
pub mod webassets;
pub mod yolo;

// P0011 传输适配层：HTTP 进 server feature，MCP 进 mcp feature；api 层无新依赖不设门。
#[cfg(feature = "mcp")]
pub mod mcp;
pub mod servectl;
#[cfg(feature = "server")]
pub mod server;
