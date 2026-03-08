//! PureWorker 桌面应用入口模块
//! 
//! 负责启动 Tauri 应用并初始化核心服务

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    pure_worker_lib::run()
}
