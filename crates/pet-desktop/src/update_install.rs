pub fn prepare_installer(url: &str) -> Result<std::path::PathBuf, String> {
    if !cfg!(windows) { return Err("请从发布页面下载此平台的更新".into()); }
    let dir = pet_core::preferences::user_dir().join("updates");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let script = dir.join("download.ps1");
    std::fs::write(&script, include_str!("../../../tools/download_update.ps1")).map_err(|e| e.to_string())?;
    let destination = dir.join("DragonBubble-update.exe");
    let mut command = std::process::Command::new("powershell.exe");
    command.args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(script).arg("-Url").arg(url).arg("-Destination").arg(&destination);
    #[cfg(windows)] { use std::os::windows::process::CommandExt; command.creation_flags(0x08000000); }
    let output = command.output().map_err(|e| e.to_string())?;
    if !output.status.success() { return Err("更新下载或 SHA256 校验失败，请检查网络后重试。".into()); }
    Ok(destination)
}
pub fn launch_installer(path: std::path::PathBuf) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let folder = exe.parent().ok_or("无法获取安装目录")?;
    std::process::Command::new(path).arg("/SILENT").arg("/NORESTART").arg("/UPDATE")
        .arg(format!("/DIR={}", folder.display())).spawn().map_err(|e| e.to_string())?;
    Ok(())
}
