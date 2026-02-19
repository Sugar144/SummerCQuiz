#[cfg(not(target_arch = "wasm32"))]
use reqwest::blocking::Client;
#[cfg(not(target_arch = "wasm32"))]
use reqwest::header::USER_AGENT;
#[cfg(not(target_arch = "wasm32"))]
use self_update::backends::github::ReleaseList;
#[cfg(not(target_arch = "wasm32"))]
use semver::Version;
#[cfg(not(target_arch = "wasm32"))]
use std::fs::File;

#[cfg(not(target_arch = "wasm32"))]
pub fn check_latest_release() -> Result<Option<String>, Box<dyn std::error::Error>> {
    let releases = ReleaseList::configure()
        .repo_owner("Sugar144")
        .repo_name("SummerCQuiz")
        .build()?
        .fetch()?;

    // versión actual del binario
    let current = Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|e| format!("parse current version: {e}"))?;

    // Elegimos la mayor versión válida por semver (ignorando tags no parseables)
    let latest = releases
        .into_iter()
        // si tienes flags como draft/prerelease, filtra aquí:
        // .filter(|r| !r.draft && !r.prerelease)
        .filter_map(|r| {
            let s = r.version.trim_start_matches('v'); // soporta tags 'v0.2.0'
            Version::parse(s).ok().map(|v| (v, s.to_string()))
        })
        .max_by(|(a, _), (b, _)| a.cmp(b)); // mayor versión

    if let Some((latest_ver, latest_str)) = latest {
        if latest_ver > current {
            return Ok(Some(latest_str)); // hay actualización
        }
    }

    Ok(None) // ya estás en la última o no hay releases parseables
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
