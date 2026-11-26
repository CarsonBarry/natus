use std::path::PathBuf;
use xcap::Monitor;

pub fn capture_primary(path: PathBuf) -> anyhow::Result<PathBuf> {
    let monitors = Monitor::all()?;
    let mon = monitors
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("no monitor"))?;
    let img = mon.capture_image()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    img.save(&path)?;
    Ok(path)
}

pub fn capture_all_monitors(dir: &std::path::Path) -> anyhow::Result<Vec<PathBuf>> {
    std::fs::create_dir_all(dir)?;
    let mut out = vec![];
    for (i, m) in Monitor::all()?.into_iter().enumerate() {
        let img = m.capture_image()?;
        let p = dir.join(format!("monitor_{}.png", i));
        img.save(&p)?;
        out.push(p);
    }
    Ok(out)
}
