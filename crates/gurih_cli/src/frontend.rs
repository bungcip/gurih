use std::path::{Path, PathBuf};
use std::process::Command;

pub fn rebuild_frontend() {
    println!("📦 Rebuilding frontend...");
    #[cfg(windows)]
    let npm_cmd = "npm.cmd";
    #[cfg(not(windows))]
    let npm_cmd = "npm";

    let web_dir = Path::new("web");
    if !web_dir.exists() {
        return;
    }

    let status = Command::new(npm_cmd)
        .arg("run")
        .arg("build")
        .current_dir(web_dir)
        .status();

    match status {
        Ok(s) if s.success() => println!("✅ Frontend rebuilt."),
        _ => eprintln!("❌ Frontend build failed."),
    }
}

pub fn ensure_frontend_built() -> Option<PathBuf> {
    let web_dir = Path::new("web");
    let dist_dir = web_dir.join("dist");

    if web_dir.exists() {
        if !dist_dir.exists() {
            println!("📦 Frontend build not found in web/dist. Attempting to build...");
            #[cfg(windows)]
            let npm_cmd = "npm.cmd";
            #[cfg(not(windows))]
            let npm_cmd = "npm";

            let install_status = Command::new(npm_cmd).arg("install").current_dir(web_dir).status();

            if let Ok(status) = install_status {
                if status.success() {
                    let build_status = Command::new(npm_cmd)
                        .arg("run")
                        .arg("build")
                        .current_dir(web_dir)
                        .status();

                    if let Ok(b_status) = build_status {
                        if !b_status.success() {
                            eprintln!("⚠️ Failed to build frontend.");
                        }
                    } else {
                        eprintln!("⚠️ Failed to run npm run build.");
                    }
                } else {
                    eprintln!("⚠️ Failed to run npm install.");
                }
            } else {
                eprintln!("⚠️ Failed to run npm.");
            }
        }

        if dist_dir.exists() {
            println!("🚀 Serving frontend from {}", dist_dir.display());
            return Some(dist_dir);
        } else {
            eprintln!("⚠️ Frontend build not found. Dashboard will not be available.");
        }
    }
    None
}
