#[cfg(not(target_arch = "wasm32"))]
use reqwest::blocking::Client;
#[cfg(not(target_arch = "wasm32"))]
use reqwest::header::USER_AGENT;
#[cfg(not(target_arch = "wasm32"))]
use self_update::backends::github::ReleaseList;
#[cfg(not(target_arch = "wasm32"))]
use std::fs::File;

#[cfg(not(target_arch = "wasm32"))]
pub fn check_latest_release() -> Result<Option<String>, Box<dyn std::error::Error>> {
    let releases = ReleaseList::configure()
        .repo_owner("Sugar144")
        .repo_name("SummerCQuiz")
        .build()?
        .fetch()?;

    if let Some(release) = releases.first() {
        let latest_version = release.version.clone();
        let current_version = env!("CARGO_PKG_VERSION").to_string();
        if latest_version != current_version {
            return Ok(Some(latest_version));
        }
    }
    Ok(None)
}

#[cfg(not(target_arch = "wasm32"))]

pub fn descargar_binario_nuevo() -> Result<(), Box<dyn std::error::Error>> {

    let releases = ReleaseList::configure()
        .repo_owner("Sugar144")
        .repo_name("SummerCQuiz")
        .build()?
        .fetch()?;

    let release = releases.first().expect("No hay releases");

    let (asset_name, local_name) = if cfg!(windows) {
        ("summer_quiz_bin.exe", "summer_quiz_bin_new.exe")
    } else {
        ("summer_quiz_bin", "summer_quiz_bin_new")
    };

    let asset = release
        .assets
        .iter()
        .find(|a| a.name == asset_name)
        .expect("No se encontró asset para el sistema actual");

    // CORREGIDO: Usa Client y añade User-Agent
    let client = Client::new();
    let mut resp = client
        .get(&asset.download_url)
        .header(USER_AGENT, "summer_quiz_updater/1.0")
        .header(reqwest::header::ACCEPT, "application/octet-stream")
        .send()?;
    let mut out = File::create(local_name)?;
    resp.copy_to(&mut out)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(local_name, perms)?;
    }

    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub fn check_latest_release() -> Result<Option<String>, Box<dyn std::error::Error>> {
    Ok(None)
}

#[cfg(target_arch = "wasm32")]
pub fn descargar_binario_nuevo() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}
