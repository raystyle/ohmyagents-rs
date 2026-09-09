pub mod agents;
pub mod archive;
pub mod caps;
pub mod deploy;
pub mod diagnose;
pub mod doctor;
pub mod fmtio;
pub mod hook;
pub mod install;
pub mod pathutil;
pub mod secretguard;
pub mod statusline;
pub mod trace;
pub mod update;
pub mod verify;
pub mod yolo;

/// 测试共享 env 互斥：动 OMA_HOME / SOPS_AGE_KEY_FILE 的测试跨模块也要
/// 互斥（各自局部锁挡不住并发互踩）。
#[cfg(test)]
pub(crate) mod testenv {
    pub static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
}
