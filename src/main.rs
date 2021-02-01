use anyhow::{Context, Result};

use gio::{
    FileExt,
    FileEnumeratorExt,
};

fn fill_coswid_tag_from_file(tag: &mut coswid::CoSWIDTag, commit: gio::File, cancel: &gio::Cancellable) -> Result<()> {
    let children = commit.enumerate_children("", gio::FileQueryInfoFlags::all(), Some(cancel))
        .context("Unable to enumerate commit children")?;
    println!("Children enum: {:?}", children);

    while let Some(file_info) = children.next_file(Some(cancel)).context("Unable to get next file")? {
        let file = children.get_child(&file_info).expect("No child file?");
        println!("File: {:?}, type: {:?}, path: {:?}", file, file_info.get_file_type(), file.get_path());

        match file_info.get_file_type() {
            gio::FileType::Directory => {
                fill_coswid_tag_from_file(tag, file, cancel)?;
            }
            gio::FileType::Regular => {
                let reader = file.read(Some(cancel))?;
                println!("Got regular file, reader: {:?}", reader);
            }
            other => println!("Got different type of file: {}", other)
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    let cancel = gio::Cancellable::new();

    // For some reason, new(path) doesn't work.
    let repopath = gio::File::new_for_path("./repo");
    let repo = ostree::Repo::new(&repopath);
    repo.open(Some(&cancel))
        .context("Failed to open ostree repository")?;
    println!("repo: {:?}", repo);

    let commit = repo.read_commit("fedora:fedora/stable/x86_64/iot", Some(&cancel))
        .context("Failed to read the commit")?;
    println!("Commit ID: {}", commit.1);

    let coswidtag = coswid::CoSWIDTag{
        // TODO: Build tag ID dynamically
        tag_id: "org.fedoraproject.iot.x86_64.stable.insert_verison_here".to_string(),
        tag_version: 0,

        corpus: Some(true),
        patch: Some(false),
        supplemental: Some(false),

        software_name: "Fedora IoT OSTree".to_string(),
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
        payload: None,
        evidence: None,

        global_attributes: Default::default(),
    };

    let mut coswidtag = coswidtag;
    fill_coswid_tag_from_file(&mut coswidtag, commit.0, &cancel)
        .context("Unable to build coswid data from ostree commit")?;
    let coswidtag = coswidtag;

    println!("Built tag: {:?}", coswidtag);

    todo!();
}
