use anyhow::{Context, Result};

use gio::{prelude::InputStreamExtManual, FileEnumeratorExt, FileExt};
use glib::Cast;
use ostree::RepoFileExt;
use sha2::{Digest, Sha256};

fn hash_file(instream: gio::InputStream) -> Result<Vec<u8>> {
    let mut hasher = Sha256::new();
    let mut reader = instream.into_read();

    let _ = std::io::copy(&mut reader, &mut hasher).context("Error copying input file to hash");

    Ok(hasher.finalize().as_slice().to_vec())
}

fn get_coswid_dir_from_file(
    repo: &ostree::Repo,
    dir: gio::File,
    cancel: Option<&gio::Cancellable>,
) -> Result<coswid::DirectoryEntry> {
    let mut files: Vec<coswid::FileEntry> = Vec::new();
    let mut dirs: Vec<coswid::DirectoryEntry> = Vec::new();

    let fs_name = match dir.get_basename() {
        Some(dir) => dir.to_str().unwrap().to_string(),
        None => "".to_string(),
    };

    println!("Handling directory {}", fs_name);

    let mut new_entry = coswid::DirectoryEntry {
        key: Some(true),
        location: None,
        fs_name,
        root: "/".to_string(),
        path_elements: Box::new(coswid::PathElementGroup {
            file: None,
            directory: None,
        }),
        global_attributes: Default::default(),
    };

    let children = dir
        .enumerate_children("", gio::FileQueryInfoFlags::all(), cancel)
        .context("Unable to enumerate commit children")?;

    while let Some(file_info) = children
        .next_file(cancel)
        .context("Unable to get next file")?
    {
        let file = children.get_child(&file_info).expect("No child file?");
        let display_path = file.get_path().unwrap();
        let display_path = display_path.to_string_lossy();

        match file_info.get_file_type() {
            gio::FileType::Directory => {
                dirs.push(
                    get_coswid_dir_from_file(repo, file, cancel).with_context(|| {
                        format!("Error building CoSWID directory entry at {}", display_path)
                    })?,
                );
            }
            gio::FileType::Regular => {
                let file: ostree::RepoFile =
                    file.downcast().expect("Could not downcast to RepoFile");
                let csum = file
                    .get_checksum()
                    .expect("Could not find checksum for file")
                    .as_str()
                    .to_string();
                let (instream, _, _) = repo
                    .load_file(&csum, cancel)
                    .with_context(|| format!("Error loading file at {}", display_path))?;
                let instream = instream.expect("Did not get input stream?");

                let digest = hash_file(instream)
                    .with_context(|| format!("Error hashing file at {}", display_path))?;

                files.push(coswid::FileEntry {
                    key: Some(true),
                    location: None,
                    fs_name: file.get_basename().unwrap().to_str().unwrap().to_string(),
                    root: "/".to_string(),
                    size: None,
                    file_version: None,
                    hash: Some((coswid::HashAlgorithm::Sha256, digest)),
                });
            }
            gio::FileType::SymbolicLink => {
                // Nothing in the spec....
            }
            other => println!("Got different type of file: {}", other),
        }
    }

    match files.len() {
        0 => {
            new_entry.path_elements.file = None;
        }
        1 => {
            new_entry.path_elements.file = Some(coswid::OneOrMany::One(files.pop().unwrap()));
        }
        _ => {
            new_entry.path_elements.file = Some(coswid::OneOrMany::Many(files));
        }
    }
    match dirs.len() {
        0 => {
            new_entry.path_elements.directory = None;
        }
        1 => {
            new_entry.path_elements.directory = Some(coswid::OneOrMany::One(dirs.pop().unwrap()));
        }
        _ => {
            new_entry.path_elements.directory = Some(coswid::OneOrMany::Many(dirs));
        }
    }

    Ok(new_entry)
}

const USAGESTR: &str = "Usage: ostree-coswid-generator <ostree-repo> <ref-name> <outfile>";
fn main() -> Result<()> {
    let mut args = std::env::args();

    // Ignore our name
    args.next();

    // Get arguments
    let repo_dir = args.next().expect(USAGESTR);
    let ref_name = args.next().expect(USAGESTR);
    let outfile_name = args.next().expect(USAGESTR);

    // Now run
    let cancel = gio::Cancellable::new();

    // For some reason, new(path) doesn't work.
    let repopath = gio::File::new_for_path(repo_dir);
    let repo = ostree::Repo::new(&repopath);
    repo.open(Some(&cancel))
        .context("Failed to open ostree repository")?;

    let commit = repo
        .read_commit(&ref_name, Some(&cancel))
        .context("Failed to read the commit")?;
    println!("Commit ID: {}", commit.1);

    let mut outfile = std::fs::File::create(&outfile_name)
        .with_context(|| format!("Error creating output file at {}", &outfile_name))?;

    let root = get_coswid_dir_from_file(&repo, commit.0, Some(&cancel))
        .context("Error building root directory entry")?;

    let coswidtag = coswid::CoSWIDTag {
        // TODO: Build tag ID dynamically
        tag_id: "org.fedoraproject.iot.x86_64.stable.insert_verison_here".to_string(),
        tag_version: 0,

        corpus: Some(true),
        patch: Some(false),
        supplemental: Some(false),

        software_name: "Fedora IoT".to_string(),
        // TODO: Fill
        software_version: Some("TODO".to_string()),
        version_scheme: None,

        media: None,

        software_meta: None,
        entity: coswid::OneOrMany::One(coswid::EntityEntry {
            entity_name: "Patrick Uiterwijk".to_string(),
            reg_id: None,
            role: coswid::OneOrMany::One(coswid::EntityRole::TagCreator),
            thumbprint: None,
            global_attributes: Default::default(),
        }),

        link: None,
        payload: Some(coswid::PayloadEntry {
            directory: Some(coswid::OneOrMany::One(root)),
            file: None,
            process: None,
            resource: None,
            global_attributes: Default::default(),
        }),
        evidence: None,

        global_attributes: Default::default(),
    };

    ciborium::ser::into_writer(&coswidtag, &mut outfile).context("Unable to serialize output file")
}
