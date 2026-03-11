fn main() {
    pure_worker_lib::export_typescript_bindings().expect("导出 TypeScript 绑定失败");
}
