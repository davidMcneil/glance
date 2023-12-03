use walkdir::WalkDir;

#[cfg(test)]
mod tests;

struct Index;

impl Index {
    fn from_directory() {
        for entry in WalkDir::new("foo") {
            let entry = entry.unwrap();
            println!("{}", entry.path().display());
        }
    }
}
