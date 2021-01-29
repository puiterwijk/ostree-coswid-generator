use gio::{
    FileExt,
    FileEnumeratorExt,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let C = &gio::Cancellable::new();

    // For some reason, new(path) doesn't work.
    let repopath = gio::File::new_for_path("/home/puiterwijk/src/ostree-coswid-generator/repo");
    let repo = ostree::Repo::new(&repopath);
    repo.open(Some(C))?;
    println!("repo: {:?}", repo);

    println!("Refs: {:?}", repo.list_refs(None, Some(C)));

    let commit = repo.read_commit("fedora:fedora/stable/x86_64/iot", Some(C))?;
    println!("Commit file: {:?}", commit.0);
    println!("Commit ID: {}", commit.1);

    let children = commit.0.enumerate_children("", gio::FileQueryInfoFlags::all(), Some(C))?;
    println!("Children enum: {:?}", children);

    while let Some(file_info) = children.next_file(Some(C))? {
        let file = children.get_child(&file_info).expect("No child file?");
        println!("File: {:?}, path: {:?}", file, file.get_path());
    }

    /*
    for object_name in repo.traverse_commit(&commit.1, 0, Some(C))? {
        println!("Type: {:?}", object_name.object_type());
        if object_name.object_type() != ostree::ObjectType::File {
            continue;
        }
        let file = repo.load_file(
            object_name.checksum(),
            Some(C),
        )?;
        println!("File: {:?}, path: {:?}", file, file.1.unwrap().get_path());
        */

    Ok(())
}
