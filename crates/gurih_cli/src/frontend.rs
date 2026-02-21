use std::path::{Path, PathBuf};
use std::process::Command;

fn get_npm_cmd() -> &'static str {
    if std::path::Path::new("web/pnpm-lock.yaml").exists() {
        if cfg!(windows) { "pnpm.cmd" } else { "pnpm" }
    } else {
        if cfg!(windows) { "npm.cmd" } else { "npm" }
    }
}

pub fn rebuild_frontend() {
    println!("📦 Rebuilding frontend...");
    let npm_cmd = get_npm_cmd();

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
        Ok(s) => eprintln!("❌ Frontend build failed with status: {}", s),
        Err(e) => eprintln!("❌ Failed to execute build command: {}", e),
    }
}

pub fn ensure_frontend_built() -> Option<PathBuf> {
    let web_dir = Path::new("web");
    let dist_dir = web_dir.join("dist");

    if web_dir.exists() {
        if !dist_dir.exists() {
            println!("📦 Frontend build not found in web/dist. Attempting to build...");
            let npm_cmd = get_npm_cmd();

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
