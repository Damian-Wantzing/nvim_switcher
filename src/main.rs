use std::{
    env,
    fs::{self, File},
    io::BufReader,
    os,
    path::Path,
    process::Command,
};

use clap::{Parser, Subcommand};
use flate2::bufread::GzDecoder;
use tar::Archive;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug, Clone)]
enum Commands {
    Download {
        #[arg()]
        version: String,
    },
    Switch {
        #[arg()]
        version: String,
    },
    Current,
    Purge {
        #[arg()]
        version: String,
    },
}

// the base url for neovim downloads
static GITHUB_BASE_URL: &str = "https://github.com/neovim/neovim/releases/download/";

fn main() {
    // parse the arguments
    let args = Args::parse();

    // which command should we run
    match args.cmd {
        Commands::Download { version } => {
            if let Err(error) = download(&version) {
                println!("{}", error);
            }
        }
        Commands::Switch { version } => {
            if let Err(error) = switch(&version) {
                println!("{}", error);
            }
        }
        Commands::Current => {
            // get the current version
            match current() {
                Ok(version) => {
                    println!("Current version: {}", version);
                }
                Err(error) => {
                    println!("{}", error);
                }
            }
        }
        Commands::Purge { version } => {
            if let Err(error) = purge(&version) {
                println!("{}", error);
            }
        }
    }
}

// Download the specified version of nvim and store it in our cache
fn download(version: &str) -> Result<Box<Path>, Box<dyn std::error::Error>> {
    // get the file path
    let path = path(version);

    // if the cache dir contains the version, there is no point in downloading it again
    if path.exists() {
        println!("Version {} already downloaded", version);

        return Ok(path);
    }

    // create the download url
    let url = GITHUB_BASE_URL.to_string() + version + "/nvim-linux64.tar.gz";

    println!("Pulling version {} of nvim from {}", version, url);

    // attempt to download the file
    let mut response = match reqwest::blocking::get(url) {
        Ok(response) => response,
        Err(_) => {
            return Err("Failed to download version".into());
        }
    };

    // was it successful
    if !response.status().is_success() {
        return Err("Failed to download version".into());
    }

    // create the file
    let mut file = match File::create(path.clone()) {
        Ok(file) => file,
        Err(_) => {
            return Err("Failed to store version".into());
        }
    };

    // write the file
    if response.copy_to(&mut file).is_err() {
        // remove the file
        let _ = fs::remove_file(path);

        return Err("Failed to store version".into());
    }

    println!("Downloaded version {} of nvim", version);

    Ok(path)
}

// Switch to the specified version of nvim
fn switch(version: &str) -> Result<(), Box<dyn std::error::Error>> {
    // is the current version the same as the one we are switching to
    if version == current()? {
        println!("Already using version {}", version);

        return Ok(());
    }

    println!("Switching to version {}", version);

    // get the path
    let path = path(version);

    // download the version if it is not already downloaded
    if !path.exists() {
        // download the version
        if download(version).is_err() {
            return Err(format!("Failed to download version {}", version).into());
        }
    }

    // remove the current version
    if let Err(_) = fs::remove_dir_all(&output_dir()) {
        return Err("Failed to remove current version".into());
    }

    // extract the version
    if let Err(_) = extract(&path, &output_dir()) {
        return Err("Failed to extract version".into());
    }

    // construct the symlink path
    let link_path = env::var("HOME").unwrap() + "/.local/";

    // turn into path
    let link = Path::new(&link_path);

    // get the output dir
    let dir = output_dir().join("nvim-linux64");

    // create symlinks for bin
    symlinks(&dir.join("bin"), &link.join("bin"))?;

    // create symlinks for lib
    symlinks(&dir.join("lib"), &link.join("lib"))?;

    // loop over the share directory and create symlinks for all the files and folders
    for entry in fs::read_dir(&dir.join("share"))? {
        let entry = entry?;

        // create symlinks
        symlinks(&entry.path(), &link.join("share").join(entry.file_name()))?;
    }

    println!("Switched to version {}", version);

    Ok(())
}

// create symlinks for all the files and folders in the directory
fn symlinks(original: &Path, output: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // loop over the original directory
    let dir = fs::read_dir(original)?;

    // loop over the entries
    for entry in dir {
        let entry = entry?;

        // get the file name
        let file_name = entry.file_name();

        // determine the name of the symlink
        let link = output.join(&file_name);

        // create the output directory if it does not exist
        if !output.exists() {
            fs::create_dir_all(&output)?;
        }

        // does the file already exist
        if link.exists() {
            // remove the file
            fs::remove_file(&link)?;
        }

        // create the symlink
        os::unix::fs::symlink(entry.path(), link.clone())?;
    }

    Ok(())
}

fn extract(file: &Path, output_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // open the file
    let file = File::open(file)?;

    // decompress the file
    let decompressed = GzDecoder::new(BufReader::new(file));

    // create the archive to read the content
    let mut archive = Archive::new(decompressed);

    // extract the content
    archive.unpack(output_dir)?;

    Ok(())
}

// remove a version from the cache
fn purge(version: &str) -> Result<(), Box<dyn std::error::Error>> {
    // get the path
    let path = path(version);

    // does the file exist
    if path.exists() {
        // remove the file
        if let Err(_) = fs::remove_file(path) {
            return Err(format!("Failed to remove version: {}", version).into());
        }
    } else {
        return Err(format!("Version {} not found", version).into());
    }

    Ok(())
}

// get the path to the version
fn path(version: &str) -> Box<Path> {
    // get the cache dir
    let cache_dir = cache_dir();

    // create the file path
    let path = cache_dir.join(format!("nvim-{}.tar.gz", version));

    // create the path
    Path::new(&path).into()
}

// get the cache directory
fn cache_dir() -> Box<Path> {
    // get the cache directory from the env var
    // with a backup to the home directory
    let mut dir =
        env::var("XDG_CACHE_HOME").unwrap_or_else(|_| env::var("HOME").unwrap() + "/.cache");

    // add our directory to the cache
    dir += "/nvim_switcher";

    // create the path
    let path = Path::new(&dir);

    // does the directory exist
    if !path.exists() {
        // create the directory
        fs::create_dir_all(&dir).unwrap();
    }

    path.into()
}

// get the output dir of the current version
fn output_dir() -> Box<Path> {
    // format the output directory
    let path = cache_dir().join("current");

    // does the directory exist
    if !path.exists() {
        // create the directory
        fs::create_dir_all(&path).unwrap();
    }

    path.into()
}

// get the current version of nvim
fn current() -> Result<String, Box<dyn std::error::Error>> {
    // get the output directory
    let output = output_dir();

    // find the nvim executable
    let nvim = output.join("nvim-linux64/bin/nvim");

    // does the file exist
    if !nvim.exists() {
        return Ok("None".to_string());
    }

    // get the version
    let output = Command::new(nvim).arg("--version").output()?;

    // convert the output to a string
    let output = String::from_utf8(output.stdout)?;

    // version number is on the first line in this syntax: NVIM v0.11.0-dev
    let version = output.lines().next().unwrap().split_whitespace().nth(1);

    // do we have a version
    match version {
        Some(version) => {
            return Ok(version.to_string());
        }
        None => {
            return Err("Failed to get version".into());
        }
    }
}
