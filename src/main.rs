use walkdir::WalkDir;

fn main() {
    // Test folder path, replace with your own path
    let folder = r"D:\coding\mindswall\example.wallpaper";

    for entry in WalkDir::new(folder)
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }

        let extension = entry
            .path()
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();

        if matches!(
            extension.as_str(),
            "jpg" | "jpeg" | "png" | "bmp"
        ) {
            println!("{}", entry.path().display());
        }
    }
}