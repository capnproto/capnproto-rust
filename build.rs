use anyhow::{anyhow, bail, Context};
use std::{
    env, fs,
    io::Cursor,
    path::{Path, PathBuf},
    process::Command,
};

// update this whenever you change the submodule pointer
const CAPNP_VERSION: &str = "0.10.2";

fn main() -> anyhow::Result<()> {
    // we're making the assumption that the executable doesn't move.
    // if we can't make this assumption, we can just include_bytes!() it and then unpack it at runtime.

    // we depend on the absolute path of the /target directory, so we need to reload
    // if the project is ever moved.
    println!("cargo:rerun-if-env-changed=PWD");

    // updated with the final path of the capnp binary if it's ever found, to be recorded
    // and consumed by capnp_import!()
    let mut capnp_path = PathBuf::new();

    // only build if it can't be detected in the $PATH
    // check if there is a capnp binary in the path that meets the version requirement
    let existing_capnp: anyhow::Result<PathBuf> = (|| {
        let bin = which::which("capnp").context("could not find a system capnp binary")?;
        let version = get_version(&bin).context(
            "could not obtain version of found binary, system capnp may be inaccessible",
        )?;

        println!("found capnp '{version}'");

        if version.trim() == format!("Cap'n Proto version {}", CAPNP_VERSION) {
            capnp_path = bin.clone();
            Ok(bin)
        } else {
            println!("cargo:warning=System version of capnp found ({}) does not meet version requirement {CAPNP_VERSION}.", &version);
            Err(anyhow!(
                "version of system capnp does not meet version requirements"
            ))?
        }
    })();

    // no capnp here, proceed to build
    if let Err(e) = existing_capnp {
        #[cfg(feature = "deny-net-fetch")]
        bail!("Couldn't find a local capnp: {}\n refusing to build", e);

        println!("Couldn't find a local capnp: {}", e);
        println!("building...");

        if cfg!(target_os = "linux") {
            // build capnproto with cmake on linux targets

            // fail with an error message: if the capnproto submodule just doesn't exist,
            // ask if the user correctly cloned the repo.
            if PathBuf::from("./capnproto")
                .as_path()
                .read_dir()
                .iter()
                .count()
                == 0
            {
                bail!(
                    "capnproto is empty - did you forget to initialize submodules before building?"
                );
            }

            // is dst consistent? might need to write this down somewhere if it isn't
            let dst = cmake::build("capnproto");
            capnp_path = dst.join("bin/capnp");
        } else if cfg!(target_os = "windows") {
            // download the release zip into $OUT_DIR
            let capnp_url =
                format!("https://capnproto.org/capnproto-c++-win32-{CAPNP_VERSION}.zip");

            let response = reqwest::blocking::get(capnp_url)?;

            if response.status() != reqwest::StatusCode::OK {
                bail!(
                    "Error downloading release archive: {} {}",
                    response.status(),
                    response.text().unwrap_or_default()
                );
            }
            println!("Download successful.");

            let capnp_dir =
                PathBuf::from(env::var("OUT_DIR").context("Cargo did not set $OUT_DIR.")?);

            // extract the release zip into $OUT_DIR
            fs::create_dir_all(&capnp_dir)?;
            let cursor = Cursor::new(response.bytes()?);
            zip_extract::extract(cursor, &capnp_dir, false)?;

            // find where capnp.exe is
            let capnp_exe =
                capnp_dir.join(format!("capnproto-tools-win32-{CAPNP_VERSION}/capnp.exe"));
            capnp_path = capnp_exe;
        } else {
            panic!("Sorry, your operating system is unsupported for building keystone.");
        }
    }

    // export the location of the finalized binary to a place that the lib.rs
    // can find it
    // this might cause problems later, but we'd need artefact deps that allow arbitrary
    // non-rust artefacts to fix it

    let out_dir = PathBuf::from(env::var("OUT_DIR").context("Cargo did not set $OUT_DIR.")?);

    fs::write(
        out_dir.join("binary_decision.rs"),
        format!("const CAPNP_BIN_PATH: &str = \"{}\";", capnp_path.display()),
    )?;

    Ok(())
}

fn get_version(executable: &Path) -> anyhow::Result<String> {
    let version = String::from_utf8(Command::new(executable).arg("--version").output()?.stdout)?;
    Ok(version)
}
