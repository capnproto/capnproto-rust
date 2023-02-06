use anyhow::{anyhow, Context};
use relative_path::RelativePathBuf;
use std::{
    env,
    fmt::Display,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

// update this whenever you change the subtree pointer
const CAPNP_VERSION: &str = "0.11.0";

enum CapnprotoAcquired {
    Locally(relative_path::RelativePathBuf),
    OnSystem(PathBuf),
}

impl Display for CapnprotoAcquired {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CapnprotoAcquired::Locally(e) => write!(f, "{}", e),
            CapnprotoAcquired::OnSystem(e) => write!(f, "{}", e.display()),
        }
    }
}

fn main() -> anyhow::Result<()> {
    // we're making the assumption that the executable is always accessible.
    // if we can't make this assumption, we can just include_bytes!() it and then unpack it at runtime.

    println!("cargo:rerun-if-changed=capnproto");

    let out_dir = PathBuf::from(
        env::var("OUT_DIR").context("Cargo did not set $OUT_DIR. this should be impossible.")?,
    );

    // updated with the final path of the capnp binary if it's ever found, to be recorded
    // and consumed by capnp_import!()
    let mut capnp_path: Option<CapnprotoAcquired> = None;

    // only build if it can't be detected in the $PATH
    // check if there is a capnp binary in the path that meets the version requirement
    let existing_capnp: anyhow::Result<PathBuf> = (|| {
        let bin = which::which("capnp").context("could not find a system capnp binary")?;
        let version = get_version(&bin).context(
            "could not obtain version of found binary, system capnp may be inaccessible",
        )?;

        println!("found capnp '{version}'");

        if version.trim() == format!("Cap'n Proto version {}", CAPNP_VERSION) {
            capnp_path = Some(CapnprotoAcquired::OnSystem(bin.clone()));
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

        // when capnproto accepts our PR, windows can fetch bin artifacts from it.
        // until then, we must build capnproto ourselves.
        capnp_path = Some(build_with_cmake(&out_dir)?);
    }

    // export the location of the finalized binary to a place that the lib.rs can find it
    // this might cause problems later, but we'd need artefact deps that allow arbitrary
    // non-rust artefacts to fix it
    fs::write(
        out_dir.join("binary_decision.rs"),
        format!(
            "const CAPNP_BIN_PATH: &str = \"{}/{}\";",
            out_dir.to_string_lossy().replace('\\', "/"),
            capnp_path.unwrap()
        ),
    )?;

    Ok(())
}

fn get_version(executable: &Path) -> anyhow::Result<String> {
    let version = String::from_utf8(Command::new(executable).arg("--version").output()?.stdout)?;
    Ok(version)
}

// build capnproto with cmake, configured for windows and linux envs
fn build_with_cmake(out_dir: &PathBuf) -> anyhow::Result<CapnprotoAcquired> {
    // is dst consistent? might need to write this down somewhere if it isn't
    let mut dst = cmake::Config::new("capnproto");

    if which::which("ninja").is_ok() {
        dst.generator("Ninja");
    }

    // it would be nice to be able to use mold

    if cfg!(target_os = "windows") {
        dst.cxxflag("/EHsc");
    }

    let dst = dst.define("BUILD_TESTING", "OFF").build();

    assert_eq!(*out_dir, dst);

    // place the capnproto binary in $OUT_DIR, next to where binary_decision.rs
    // is intended to go
    if cfg!(target_os = "windows") {
        Ok(CapnprotoAcquired::Locally(RelativePathBuf::from(
            "bin/capnp.exe",
        )))
    } else if cfg!(target_os = "linux") {
        Ok(CapnprotoAcquired::Locally(RelativePathBuf::from(
            "bin/capnp",
        )))
    } else {
        panic!("Sorry, capnp_import does not support your operating system.");
    }
}
