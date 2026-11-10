use std::fs;
use std::path::Path;

use super::{Result, ctx, now, open_store};

/// Writes everything stored to the backup file `path`, replacing it.
pub fn export(path: &Path) -> Result<()> {
    let backup = open_store()?
        .export_backup(now()?)
        .map_err(ctx("exporting"))?;
    fs::write(path, backup).map_err(ctx(format!("writing {}", path.display())))?;
    println!("exported to {}", path.display());
    Ok(())
}

/// Merges the backup file `path` into what is stored.
pub fn import(path: &Path) -> Result<()> {
    let backup = fs::read_to_string(path).map_err(ctx(format!("reading {}", path.display())))?;
    let imported = open_store()?
        .import_backup(&backup)
        .map_err(ctx(format!("importing {}", path.display())))?;
    println!(
        "imported {} favorites, {} visits and {} resume points; settings restored",
        imported.favorites, imported.visits, imported.resume_points
    );
    Ok(())
}
